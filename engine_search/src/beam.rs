use engine_core::{game::Game, ruleset::AttackContext};
use engine_core::placement::Move;
use engine_eval::Model;
use engine_nav::{buffer::MoveBuffer, movegen};
use rayon::prelude::*;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::bag::extend_queue;
use crate::config::{SearchConfig, SearchResult};
use crate::transposition::{get_zobrist_keys, TranspositionTable, ZobristKeys, DEFAULT_TT_SIZE};

pub struct Beam {
    pub config: SearchConfig,
}

thread_local! {
    static MOVE_BUFFER: RefCell<MoveBuffer> = RefCell::new(MoveBuffer::new());
}

impl Beam {
    #[must_use]
    pub fn search(&self, game: &Game) -> Option<SearchResult> {
        if self.config.width == 0 || self.config.depth == 0 {
            return None;
        }

        let root = if self.config.extend_queue_7bag {
            let mut next = *game;
            next.queue = extend_queue(&next.queue, next.current, next.hold);
            next
        } else {
            *game
        };

        let max_depth = self.config.depth.min(root.queue_len() + 1);
        if max_depth == 0 {
            return None;
        }

        let max_width = self.config.width;
        let model = &self.config.model;
        let zobrist_keys = get_zobrist_keys();
        let tt = self
            .config
            .use_tt
            .then(|| Arc::new(Mutex::new(TranspositionTable::new(DEFAULT_TT_SIZE))));

        let Some(time_budget_ms) = self.config.time_budget_ms else {
            return run_iteration(
                &root,
                model,
                max_depth,
                max_width,
                self.config.futility_delta,
                self.config.quiescence_max_extensions,
                self.config.quiescence_beam_fraction,
                zobrist_keys,
                tt.as_ref(),
            );
        };

        if max_width == 0 {
            return None;
        }

        let mut width = 200.min(max_width);
        let mut best: Option<SearchResult> = None;
        let start = Instant::now();
        let budget = Duration::from_millis(time_budget_ms);

        loop {
            if let Some(table) = tt.as_ref()
                && let Ok(mut guard) = table.lock() {
                    guard.clear();
                }

            if let Some(result) = run_iteration(
                &root,
                model,
                max_depth,
                width,
                self.config.futility_delta,
                self.config.quiescence_max_extensions,
                self.config.quiescence_beam_fraction,
                zobrist_keys,
                tt.as_ref(),
            )
                && best.as_ref().is_none_or(|prev| result.score > prev.score) {
                    best = Some(result);
                }

            if width >= max_width || start.elapsed() >= budget {
                break;
            }

            width = (width * 2).min(max_width);
        }

        best
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
#[inline]
fn run_iteration(
    game: &Game,
    model: &Model,
    max_depth: usize,
    width: usize,
    futility_delta: f64,
    quiescence_max_extensions: usize,
    quiescence_beam_fraction: f64,
    zobrist_keys: &ZobristKeys,
    tt: Option<&Arc<Mutex<TranspositionTable>>>,
) -> Option<SearchResult> {
    let mut beam = expand_root(game, model, width, max_depth.saturating_sub(1), zobrist_keys, tt);
    if beam.is_empty() {
        return None;
    }

    apply_futility_pruning(&mut beam, futility_delta);
    select_top_k(&mut beam, width);

    let mut next = Vec::new();

    for depth_idx in 1..max_depth {
        let remaining_depth = max_depth.saturating_sub(depth_idx + 1);
        let chunks: Vec<TopK> = beam
            .par_iter()
            .map(|node| {
                expand_node_topk(node, model, width, remaining_depth, zobrist_keys, tt)
            })
            .collect();

        next.clear();
        let total = chunks.iter().map(TopK::len).sum();
        next.reserve(total);
        for chunk in chunks {
            next.extend(chunk.into_vec());
        }

        if next.is_empty() {
            break;
        }

        apply_futility_pruning(&mut next, futility_delta);
        select_top_k(&mut next, width);
        beam.clear();
        std::mem::swap(&mut beam, &mut next);
    }

    apply_quiescence_extensions(
        &mut beam,
        model,
        max_depth,
        width,
        quiescence_max_extensions,
        quiescence_beam_fraction,
        zobrist_keys,
        tt,
    );

    let best = beam.first()?;
    Some(SearchResult {
        best_move: best.root_move,
        score: best.score,
    })
}

#[derive(Clone)]
struct BeamNode {
    game: Game,
    score: f64,
    root_move: Move,
    last_lines: u8,
}

impl BeamNode {
    fn is_loud(&self) -> bool {
        self.game.combo > 0 || self.game.b2b > 0 || self.last_lines > 0
    }
}

struct TopK {
    k: usize,
    data: Vec<BeamNode>,
    min_score: f64,
}

impl TopK {
    fn new(k: usize) -> Self {
        Self {
            k,
            data: Vec::new(),
            min_score: f64::NEG_INFINITY,
        }
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn push(&mut self, node: BeamNode) {
        if self.k == 0 {
            return;
        }

        if self.data.len() < self.k {
            self.min_score = if self.data.is_empty() {
                node.score
            } else {
                self.min_score.min(node.score)
            };
            self.data.push(node);
            return;
        }

        if node.score <= self.min_score {
            return;
        }

        let mut worst_idx = 0;
        let mut worst_score = self.data[0].score;
        for (i, n) in self.data.iter().enumerate().skip(1) {
            if n.score < worst_score {
                worst_score = n.score;
                worst_idx = i;
            }
        }

        self.data[worst_idx] = node;
        self.recompute_min();
    }

    fn recompute_min(&mut self) {
        self.min_score = self
            .data
            .iter()
            .map(|n| n.score)
            .fold(f64::INFINITY, f64::min);
    }

    fn into_vec(self) -> Vec<BeamNode> {
        self.data
    }
}

fn select_top_k(nodes: &mut Vec<BeamNode>, k: usize) {
    if k == 0 {
        nodes.clear();
        return;
    }
    if nodes.len() <= k {
        nodes.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
        return;
    }

    nodes.select_nth_unstable_by(k, |a, b| b.score.total_cmp(&a.score));
    nodes[..k].sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
    nodes.truncate(k);
}


fn expand_root(
    game: &Game,
    model: &Model,
    width: usize,
    remaining_depth: usize,
    zobrist_keys: &ZobristKeys,
    tt: Option<&Arc<Mutex<TranspositionTable>>>,
) -> Vec<BeamNode> {
    let mut topk = TopK::new(width);
    MOVE_BUFFER.with(|cell| {
        let mut moves = cell.borrow_mut();
        generate_moves_into(game, &mut moves);
        for m in moves.iter() {
            if let Some((next, ctx, _)) = apply_move(game, *m) {
                let score = eval_score(model, &next, *m, &ctx, remaining_depth, zobrist_keys, tt);
                topk.push(BeamNode {
                    game: next,
                    score,
                    root_move: *m,
                    last_lines: ctx.lines,
                });
            }
        }
    });
    topk.into_vec()
}

fn expand_node_topk(
    node: &BeamNode,
    model: &Model,
    width: usize,
    remaining_depth: usize,
    zobrist_keys: &ZobristKeys,
    tt: Option<&Arc<Mutex<TranspositionTable>>>,
) -> TopK {
    let mut topk = TopK::new(width);
    MOVE_BUFFER.with(|cell| {
        let mut moves = cell.borrow_mut();
        generate_moves_into(&node.game, &mut moves);
        for m in moves.iter() {
            if let Some((next, ctx, _)) = apply_move(&node.game, *m) {
                let score = eval_score(model, &next, *m, &ctx, remaining_depth, zobrist_keys, tt);
                topk.push(BeamNode {
                    game: next,
                    score,
                    root_move: node.root_move,
                    last_lines: ctx.lines,
                });
            }
        }
    });
    topk
}

fn eval_score(
    model: &Model,
    game: &Game,
    mv: Move,
    ctx: &AttackContext,
    remaining_depth: usize,
    zobrist_keys: &ZobristKeys,
    tt: Option<&Arc<Mutex<TranspositionTable>>>,
) -> f64 {
    let board_score = if let Some(table) = tt.as_ref() {
        let depth = remaining_depth.min(u8::MAX as usize) as u8;
        let hash = zobrist_keys.hash_game(game);
        if let Ok(mut guard) = table.lock() {
            if let Some(score) = guard.probe(hash, depth) {
                score
            } else {
                let score = model.board.eval(game);
                guard.store(hash, depth, score);
                score
            }
        } else {
            model.board.eval(game)
        }
    } else {
        model.board.eval(game)
    };

    model.board_weight * board_score + model.active_weight * model.active.eval(&mv, ctx)
}

fn generate_moves_into(game: &Game, moves: &mut MoveBuffer) {
    moves.clear();
    movegen::generate(&game.board, moves, game.current, false);

    if let Some(hold) = game.hold {
        if hold != game.current {
            movegen::generate(&game.board, moves, hold, false);
        }
    } else if let Some(next) = game.queue_piece(0)
        && next != game.current
    {
        movegen::generate(&game.board, moves, next, false);
    }
}

fn apply_futility_pruning(nodes: &mut Vec<BeamNode>, futility_delta: f64) {
    if nodes.len() <= 1 {
        return;
    }

    let delta = futility_delta.max(0.0);
    if delta == 0.0 {
        return;
    }

    let best_score = nodes
        .iter()
        .map(|node| node.score)
        .fold(f64::NEG_INFINITY, f64::max);
    let cutoff = best_score - delta;
    nodes.retain(|node| node.score >= cutoff);
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn apply_quiescence_extensions(
    beam: &mut Vec<BeamNode>,
    model: &Model,
    max_depth: usize,
    width: usize,
    q_max: usize,
    q_fraction: f64,
    zobrist_keys: &ZobristKeys,
    tt: Option<&Arc<Mutex<TranspositionTable>>>,
) {
    if q_max == 0 || q_fraction <= 0.0 || beam.is_empty() {
        return;
    }

    let q_beam_width = ((width as f64) * q_fraction).ceil() as usize;
    if q_beam_width == 0 {
        return;
    }

    let mut q_beam: Vec<BeamNode> = beam.iter().filter(|n| n.is_loud()).cloned().collect();
    if q_beam.is_empty() {
        return;
    }

    q_beam.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
    q_beam.truncate(q_beam_width);

    for ext in 0..q_max {
        let child_depth = max_depth.saturating_sub(1) + ext + 2;
        let remaining_depth = max_depth.saturating_sub(child_depth.min(max_depth));
        let chunks: Vec<TopK> = q_beam
            .par_iter()
            .map(|node| {
                expand_node_topk(node, model, q_beam_width, remaining_depth, zobrist_keys, tt)
            })
            .collect();

        let mut next_q = Vec::new();
        let total = chunks.iter().map(TopK::len).sum();
        next_q.reserve(total);
        for chunk in chunks {
            next_q.extend(chunk.into_vec());
        }

        if next_q.is_empty() {
            break;
        }

        select_top_k(&mut next_q, q_beam_width);
        for node in &next_q {
            if !node.is_loud() {
                beam.push(node.clone());
            }
        }

        q_beam = next_q.into_iter().filter(BeamNode::is_loud).collect();
        if q_beam.is_empty() {
            break;
        }
    }

    beam.extend(q_beam);
    beam.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
}

fn apply_move(game: &Game, m: Move) -> Option<(Game, AttackContext, u8)> {
    if !has_queue_for_move(game, m) {
        return None;
    }

    let mut next = *game;
    let (ctx, outgoing) = next.advance(&m);
    Some((next, ctx, outgoing))
}

fn has_queue_for_move(game: &Game, m: Move) -> bool {
    let hold_used = game.infer_hold_used_for_piece(m.piece());
    if hold_used {
        if game.hold.is_some() {
            game.queue_len() >= 1
        } else {
            game.queue_len() >= 2
        }
    } else {
        game.queue_len() >= 1
    }
}
