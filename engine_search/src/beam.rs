use engine_core::{game::Game, ruleset::AttackContext};
use engine_core::placement::Move;
use engine_eval::Model;
use engine_nav::{buffer::MoveBuffer, movegen};

use crate::config::{SearchConfig, SearchResult};

pub struct Beam {
    pub config: SearchConfig,
}

impl Beam {
    #[must_use] 
    pub fn search(&self, game: &Game) -> Option<SearchResult> {
        if self.config.width == 0 || self.config.depth == 0 {
            return None;
        }

        let mut beam = expand_root(game, &self.config.model);
        if beam.is_empty() {
            return None;
        }

        beam.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
        beam.truncate(self.config.width);

        for _ in 1..self.config.depth {
            let mut next = Vec::new();
            for node in &beam {
                expand_node(node, &self.config.model, &mut next);
            }

            if next.is_empty() {
                break;
            }

            next.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
            next.truncate(self.config.width);
            beam = next;
        }

        let best = beam.first()?;
        Some(SearchResult {
            best_move: best.root_move,
            score: best.score,
        })
    }
}

#[derive(Clone)]
struct BeamNode {
    game: Game,
    score: f64,
    root_move: Move,
}


fn expand_root(game: &Game, model: &Model) -> Vec<BeamNode> {
    let mut nodes = Vec::new();
    let moves = generate_moves(game);
    for m in moves.iter() {
        if let Some((next, ctx, _)) = apply_move(game, *m) {
            let score = model.eval(&next, m, &ctx);
            nodes.push(BeamNode {
                game: next,
                score,
                root_move: *m,
            });
        }
    }
    nodes
}

fn expand_node(node: &BeamNode, model: &Model, out: &mut Vec<BeamNode>) {
    let moves = generate_moves(&node.game);
    for m in moves.iter() {
        if let Some((next, ctx, _)) = apply_move(&node.game, *m) {
            let score = model.eval(&next, m, &ctx);
            out.push(BeamNode {
                game: next,
                score,
                root_move: node.root_move,
            });
        }
    }
}

fn generate_moves(game: &Game) -> MoveBuffer {
    let mut moves = MoveBuffer::new();
    movegen::generate(&game.board, &mut moves, game.current, false);

    if let Some(hold) = game.hold {
        if hold != game.current {
            movegen::generate(&game.board, &mut moves, hold, false);
        }
    } else if let Some(next) = game.queue_piece(0)
        && next != game.current
    {
        movegen::generate(&game.board, &mut moves, next, false);
    }

    moves
}

fn apply_move(game: &Game, m: Move) -> Option<(Game, AttackContext, u8)> {
    if !has_queue_for_move(game, m) {
        return None;
    }

    let mut next = game.clone();
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
