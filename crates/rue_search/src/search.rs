use std::time::{Duration, Instant};

use rue_core::{game::Game, placement::Move};
use rue_eval::weights::Weights;
use rustc_hash::FxHashMap;

use crate::config::{
    SearchConfig, SearchExpansionContext, SearchIterationParams, SearchNode, SearchResult,
    SearchResultFull,
};
use rayon::prelude::*;

use crate::expand::{expand_node, expand_root};

/// Run beam search from a game state.
///
/// Returns `None` when no legal move exists.
pub fn beam_search<const N: usize, W: Weights + Sync>(
    game: &Game<N>,
    config: &SearchConfig,
    weights: &W,
) -> Option<SearchResult<N>> {
    beam_search_forced(game, config, weights, None).map(|full| full.best)
}

/// Run beam search returning full results including per-root-move scores.
pub fn beam_search_with_scores<const N: usize, W: Weights + Sync>(
    game: &Game<N>,
    config: &SearchConfig,
    weights: &W,
) -> Option<SearchResultFull<N>> {
    beam_search_forced(game, config, weights, None)
}

/// Run beam search with optional forced root move.
///
/// When `forced` is `Some`, that move is protected from futility pruning
/// and beam truncation — it always survives to the final beam so its score
/// appears in `root_scores`.
pub fn beam_search_forced<const N: usize, W: Weights + Sync>(
    game: &Game<N>,
    config: &SearchConfig,
    weights: &W,
    forced: Option<Move>,
) -> Option<SearchResultFull<N>> {
    let max_depth = config.depth.min(game.queue.len());
    if max_depth == 0 {
        return None;
    }

    if config.time_budget_ms.is_none() {
        let mut tt = if config.futility_delta > 0.0 || config.depth > 4 {
            Some(FxHashMap::default())
        } else {
            None
        };
        let params = SearchIterationParams {
            game,
            config,
            weights,
            max_depth,
            beam_width: config.beam_width,
            tt: &mut tt,
            forced_root_move: forced,
        };
        return run_beam_search_iteration(params);
    }

    let max_width = config.beam_width;
    if max_width == 0 {
        return None;
    }

    let mut width = 200.min(max_width);
    let mut best_full: Option<SearchResultFull<N>> = None;
    let start = Instant::now();
    let time_budget = config.time_budget_ms.map(Duration::from_millis);
    let mut last_iter_duration = Duration::ZERO;

    loop {
        if let Some(budget) = time_budget {
            let elapsed = start.elapsed();
            // The next iteration doubles the beam width, so it will cost
            // roughly 2x the previous one.  Refuse to start if that would
            // push the total past the budget.
            if elapsed + last_iter_duration * 2 >= budget {
                break;
            }
        }

        let iter_start = Instant::now();

        let mut tt = Some(FxHashMap::default());
        let params = SearchIterationParams {
            game,
            config,
            weights,
            max_depth,
            beam_width: width,
            tt: &mut tt,
            forced_root_move: forced,
        };

        if let Some(full) = run_beam_search_iteration(params) {
            let should_replace = best_full
                .as_ref()
                .is_none_or(|prev| full.best.best.score > prev.best.best.score);

            if should_replace {
                best_full = Some(full);
            }
        }

        last_iter_duration = iter_start.elapsed();

        if width >= max_width {
            break;
        }

        width = (width * 2).min(max_width);
    }

    best_full
}

#[allow(clippy::needless_pass_by_value)]
fn run_beam_search_iteration<const N: usize, W: Weights + Sync>(
    params: SearchIterationParams<'_, W, N>,
) -> Option<SearchResultFull<N>> {
    let mut ctx = SearchExpansionContext {
        config: params.config,
        weights: params.weights,
        remaining_depth: params.max_depth.saturating_sub(1),
        tt: params.tt,
    };

    let mut beam = expand_root(params.game, &mut ctx);
    if beam.is_empty() {
        return None;
    }

    apply_futility_pruning(&mut beam, params.config.futility_delta, params.forced_root_move);
    beam.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
    truncate_search_beam(&mut beam, params.beam_width, params.forced_root_move);

    for depth_idx in 0..params.max_depth.saturating_sub(1) {
        let child_depth = depth_idx + 2;
        ctx.remaining_depth = params.max_depth.saturating_sub(child_depth);

        let config = ctx.config;
        let weights = ctx.weights;
        let remaining_depth = ctx.remaining_depth;

        let mut next_beam: Vec<SearchNode<N>> = beam
            .par_iter()
            .flat_map_iter(|node| {
                let mut local_tt = None;
                let mut local_ctx = SearchExpansionContext {
                    config,
                    weights,
                    remaining_depth,
                    tt: &mut local_tt,
                };
                let mut out = Vec::new();
                expand_node(node, &mut local_ctx, &mut out);
                out
            })
            .collect();

        if next_beam.is_empty() {
            break;
        }

        apply_futility_pruning(
            &mut next_beam,
            params.config.futility_delta,
            params.forced_root_move,
        );
        next_beam.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
        truncate_search_beam(&mut next_beam, params.beam_width, params.forced_root_move);
        beam = next_beam;
    }

    // Quiescence extensions: extend loud nodes past the normal depth boundary
    // so investment moves (mid-combo, active B2B) resolve before evaluation.
    let q_max = params.config.quiescence_max_extensions;
    let q_beam_width =
        ((params.beam_width as f64) * params.config.quiescence_beam_fraction).ceil() as usize;
    if q_max > 0 && q_beam_width > 0 {
        let main_depth = params.max_depth.saturating_sub(1);
        let loud_nodes: Vec<SearchNode<N>> = beam.iter().filter(|n| n.is_loud()).cloned().collect();

        if !loud_nodes.is_empty() {
            let mut q_beam = loud_nodes;
            q_beam.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
            q_beam.truncate(q_beam_width);

            let config = ctx.config;
            let weights = ctx.weights;

            for ext in 0..q_max {
                let child_depth = main_depth + ext + 2;
                let remaining_depth = params
                    .max_depth
                    .saturating_sub(child_depth.min(params.max_depth));

                let mut next_q: Vec<SearchNode<N>> = q_beam
                    .par_iter()
                    .flat_map_iter(|node| {
                        let mut local_tt = None;
                        let mut local_ctx = SearchExpansionContext {
                            config,
                            weights,
                            remaining_depth,
                            tt: &mut local_tt,
                        };
                        let mut out = Vec::new();
                        expand_node(node, &mut local_ctx, &mut out);
                        out
                    })
                    .collect();

                if next_q.is_empty() {
                    break;
                }

                next_q.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
                next_q.truncate(q_beam_width);

                for node in &next_q {
                    if !node.is_loud() {
                        beam.push(node.clone());
                    }
                }

                q_beam = next_q.into_iter().filter(SearchNode::is_loud).collect();
                if q_beam.is_empty() {
                    break;
                }
            }

            beam.extend(q_beam);
            beam.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
        }
    }

    let best = beam.first()?;
    let result = SearchResult {
        best: best.clone(),
    };

    let mut root_scores: Vec<(Move, f64)> = Vec::new();
    for node in &beam {
        let raw = node.root_move;
        match root_scores.iter_mut().find(|entry| entry.0 == raw) {
            Some(entry) => {
                if node.score > entry.1 {
                    entry.1 = node.score;
                }
            }
            None => root_scores.push((raw, node.score)),
        }
    }
    root_scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    let position_complexity = compute_position_complexity(&root_scores);

    Some(SearchResultFull {
        best: result,
        root_scores,
        position_complexity,
        board_score: best.board_score,
        attack_score: best.attack_score,
        chain_score: best.chain_score,
        path_attack: best.path_attack,
        path_chain: best.path_chain,
    })
}

/// Compute position complexity: variance of top-10 root move scores.
fn compute_position_complexity(root_scores: &[(Move, f64)]) -> f64 {
    let count = root_scores.len().min(10);
    if count < 2 {
        return 0.0;
    }
    let scores: Vec<f64> = root_scores.iter().take(10).map(|(_, s)| *s).collect();
    let mean = scores.iter().sum::<f64>() / count as f64;
    scores
        .iter()
        .map(|s| (s - mean).powi(2))
        .sum::<f64>()
        / count as f64
}

/// Truncate beam to `max_size`, protecting a forced root move from truncation.
fn truncate_search_beam<const N: usize>(
    beam: &mut Vec<SearchNode<N>>,
    max_size: usize,
    forced: Option<Move>,
) {
    if beam.len() <= max_size {
        return;
    }

    // Extract forced node before truncation so it can't be lost
    let forced_node = forced.and_then(|fm| {
        let idx = beam.iter().position(|n| n.root_move == fm);
        idx.map(|i| beam.swap_remove(i))
    });

    beam.truncate(max_size);

    // Re-insert forced node, evicting worst survivor if needed
    if let Some(node) = forced_node {
        let already_present = beam.iter().any(|n| n.root_move == node.root_move);
        if !already_present {
            if beam.len() >= max_size {
                beam.pop(); // evict worst (last after sort)
            }
            beam.push(node);
        }
    }
}

fn apply_futility_pruning<const N: usize>(
    nodes: &mut Vec<SearchNode<N>>,
    futility_delta: f64,
    forced: Option<Move>,
) {
    if nodes.is_empty() || futility_delta <= 0.0 {
        return;
    }

    // Extract forced move node before pruning
    let forced_node = forced.and_then(|fm| {
        let idx = nodes.iter().position(|n| n.root_move == fm);
        idx.map(|i| nodes.swap_remove(i))
    });

    let best_score = nodes
        .iter()
        .map(|node| node.score)
        .fold(f64::NEG_INFINITY, f64::max);
    let cutoff = best_score - futility_delta;

    nodes.retain(|node| node.score >= cutoff);

    // Re-insert forced move node unconditionally
    if let Some(forced_node) = forced_node {
        let already_present = nodes
            .iter()
            .any(|n| n.root_move == forced_node.root_move);
        if !already_present {
            nodes.push(forced_node);
        }
    }
}

#[cfg(test)]
mod tests {
    use rue_core::board::Board;
    use rue_core::game::garbage::GarbageQueue;
use rue_core::game::ruleset::SEASON_2;
    use rue_core::piece::Piece;

    use rue_core::rng::Rng;
use rue_eval::simple::Simple;

    use super::*;

    const N: usize = 7;

    fn empty_game(queue: Vec<Piece>) -> Game<N> {
        Game {
            board: Board::EMPTY,
            hold: None,
            queue,
            garbage_queue: GarbageQueue::new(),
            b2b_count: None,
            combo_count: None,
            ruleset: SEASON_2,
            rng: Rng::new(),
        }
    }

    fn zero_weights() -> Simple {
        Simple {
            b2b: 0.0,
            height: 0.0,
            height_half: 0.0,
            height_three_quarters: 0.0,
            bumpiness: 0.0,
            bumpiness_sq: 0.0,
            cell_coveredness: 0.0,
            holes: 0.0,
            row_transitions: 0.0,
            active: [[0.0; 3]; 5],
            combo: 0.0,
            sent: 0.0,
            well_col: [0.0; 10],
            well_depth: 0.0,
            garbage: 0.0,
        }
    }

    #[test]
    fn empty_board_finds_move() {
        let game = empty_game(vec![Piece::T, Piece::I]);
        let config = SearchConfig::default();
        let weights = zero_weights();

        let result = beam_search(&game, &config, &weights);
        assert!(result.is_some(), "should find a move on empty board");
        let res = result.unwrap();
        assert!(
            !res.best.path.is_empty(),
            "PV should have at least one move"
        );
    }

    #[test]
    fn depth_1_returns_single_move_pv() {
        let game = empty_game(vec![Piece::S, Piece::Z]);
        let config = SearchConfig {
            beam_width: 50,
            depth: 1,
            ..SearchConfig::default()
        };
        let weights = zero_weights();

        let result = beam_search(&game, &config, &weights);
        assert!(result.is_some(), "depth-1 should find a move");
        assert_eq!(
            result.unwrap().best.path.len(),
            1,
            "depth-1 PV must have length 1"
        );
    }

    #[test]
    fn beam_width_respected() {
        let game = empty_game(vec![Piece::T, Piece::I, Piece::O, Piece::L]);
        let narrow = SearchConfig {
            beam_width: 3,
            depth: 3,
            ..SearchConfig::default()
        };
        let weights = zero_weights();

        let result = beam_search(&game, &narrow, &weights);
        assert!(result.is_some(), "narrow beam should still find something");
    }

    #[test]
    fn futility_pruning_removes_low_scores() {
        let mut nodes = vec![
            SearchNode {
                game: empty_game(vec![Piece::T]),
                score: 100.0,
                root_move: Move::new(
                    Piece::T,
                    rue_core::rotation::Rotation::North,
                    0,
                    0,
                    rue_core::spin::Spin::None,
                ),
                root_hold_used: false,
                path: vec![],
                attack_score: 0.0,
                chain_score: 0.0,
                board_score: 0.0,
                path_attack: 0.0,
                path_chain: 0.0,
            },
            SearchNode {
                game: empty_game(vec![Piece::T]),
                score: 50.0,
                root_move: Move::new(
                    Piece::T,
                    rue_core::rotation::Rotation::North,
                    1,
                    0,
                    rue_core::spin::Spin::None,
                ),
                root_hold_used: false,
                path: vec![],
                attack_score: 0.0,
                chain_score: 0.0,
                board_score: 0.0,
                path_attack: 0.0,
                path_chain: 0.0,
            },
            SearchNode {
                game: empty_game(vec![Piece::T]),
                score: 10.0,
                root_move: Move::new(
                    Piece::T,
                    rue_core::rotation::Rotation::North,
                    2,
                    0,
                    rue_core::spin::Spin::None,
                ),
                root_hold_used: false,
                path: vec![],
                attack_score: 0.0,
                chain_score: 0.0,
                board_score: 0.0,
                path_attack: 0.0,
                path_chain: 0.0,
            },
        ];

        apply_futility_pruning(&mut nodes, 60.0, None);

        assert_eq!(
            nodes.len(),
            2,
            "score 10 should be pruned (cutoff = 100 - 60 = 40)"
        );
        assert!(nodes.iter().all(|n| n.score >= 40.0));
    }

    #[test]
    fn full_board_returns_none() {
        let mut board = Board::<N>::EMPTY;
        for y in 0..Board::<N>::H {
            for x in 0..10 {
                board.set(x, y);
            }
        }
        let game = Game {
            board,
            hold: None,
            queue: vec![Piece::I],
            garbage_queue: GarbageQueue::new(),
            b2b_count: None,
            combo_count: None,
            ruleset: SEASON_2,
            rng: Rng::new(),
        };
        let config = SearchConfig::default();
        let weights = zero_weights();

        let result = beam_search(&game, &config, &weights);
        assert!(result.is_none(), "full board should have no legal moves");
    }

    #[test]
    fn root_scores_match_best_move() {
        let game = empty_game(vec![Piece::T, Piece::I]);
        let config = SearchConfig {
            beam_width: 200,
            depth: 2,
            ..SearchConfig::default()
        };
        let weights = zero_weights();

        let result = beam_search_with_scores(&game, &config, &weights);
        assert!(result.is_some());
        let res = result.unwrap();

        assert!(
            !res.root_scores.is_empty(),
            "should have at least one root score"
        );
        assert_eq!(
            res.root_scores[0].0, res.best.best.root_move,
            "top root_score must match best root_move"
        );
    }
}
