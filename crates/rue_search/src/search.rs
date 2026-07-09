use std::sync::{Arc, Mutex};

use rayon::prelude::*;
use rue_core::{game::Game, placement::Move};
use rue_eval::weights::Weights;
use rustc_hash::FxHashSet;

use crate::config::SearchConfig;
use crate::expand::{expand_node, expand_root};

/// A node in the search tree.
#[derive(Clone)]
pub struct Node<const N: usize> {
    /// The game state at this node.
    pub game: Game<N>,
    /// Score assigned by the evaluation function.
    pub score: f64,
    /// The root-level move that originated this path.
    pub root_move: Move,
    /// Sequence of moves from root to this node.
    pub path: Arc<Vec<Move>>,
}

/// Result of a beam search.
pub struct SearchResult<const N: usize> {
    /// The best leaf node found.
    pub best: Node<N>,
    /// Best score per distinct root move, sorted descending.
    pub root_scores: Vec<(Move, f64)>,
}

/// Run beam search from a game state.
///
/// Returns `None` when no legal move exists.
pub fn beam_search<const N: usize>(
    game: &Game<N>,
    config: &SearchConfig,
    weights: &(impl Weights + Sync),
) -> Option<SearchResult<N>> {
    // Each tick consumes one piece from the queue, so total depth is bounded
    // by the number of pieces available.
    let max_depth = config.depth.min(game.queue.len());
    if max_depth == 0 {
        return None;
    }

    let mut beam = expand_root(game, weights, &Mutex::new(FxHashSet::default()));
    if beam.is_empty() {
        return None;
    }

    sort_prune_truncate(&mut beam, config);

    let mut candidates = Vec::new();

    for _depth in 1..max_depth {
        candidates.clear();

        let seen = Mutex::new(FxHashSet::default());
        let batches: Vec<Vec<Node<N>>> = beam
            .par_iter()
            .map(|node| expand_node(node, weights, &seen))
            .collect();

        for batch in batches {
            candidates.extend(batch);
        }

        if candidates.is_empty() {
            break;
        }

        sort_prune_truncate(&mut candidates, config);
        std::mem::swap(&mut beam, &mut candidates);
    }

    let best = beam.first()?.clone();
    let mut root_scores: Vec<(Move, f64)> = Vec::new();

    for node in &beam {
        let root = node.root_move;
        let score = node.score;
        match root_scores.iter_mut().find(|(m, _)| *m == root) {
            Some((_, s)) => {
                if score > *s {
                    *s = score;
                }
            }
            None => root_scores.push((root, score)),
        }
    }

    root_scores.sort_by(|a, b| b.1.total_cmp(&a.1));

    Some(SearchResult { best, root_scores })
}

/// Sort nodes descending by score, apply futility pruning, then truncate to beam width.
///
/// Uses `select_nth_unstable_by` to find the top-k in O(n) time, then sorts
/// only the survivors (O(k log k)). Full sort is O(n log n) — this matters
/// when the candidate pool is large (branching × `beam_width`).
fn sort_prune_truncate<const N: usize>(nodes: &mut Vec<Node<N>>, config: &SearchConfig) {
    let k = config.beam_width.min(nodes.len());
    if k == 0 {
        nodes.clear();
        return;
    }

    // Partition so that the top-k elements occupy nodes[..k] (unsorted).
    nodes.select_nth_unstable_by(k - 1, |a, b| b.score.total_cmp(&a.score));

    if config.futility_delta > 0.0 {
        let best = nodes[..k]
            .iter()
            .map(|n| n.score)
            .max_by(f64::total_cmp)
            .unwrap();
        let cutoff = best - config.futility_delta;
        nodes.retain(|n| n.score >= cutoff);
    }

    // Sort the top portion so that beam[0] is the best node.
    let k = config.beam_width.min(nodes.len());
    if k > 1 {
        nodes[..k].sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
    }
    nodes.truncate(config.beam_width);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rue_core::board::Board;
    use rue_core::game::ruleset::SEASON_2;
    use rue_core::piece::Piece;
    use rue_eval::simple::Simple;

    use super::*;

    const N: usize = 7;

    fn empty_game(queue: Vec<Piece>) -> Game<N> {
        Game {
            board: Board::EMPTY,
            garbage_row: 0,
            hold: None,
            queue,
            garbage_queue: vec![],
            b2b_count: None,
            combo_count: None,
            ruleset: SEASON_2,
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
            Node {
                game: empty_game(vec![Piece::T]),
                score: 100.0,
                root_move: Move::new(
                    Piece::T,
                    rue_core::rotation::Rotation::North,
                    0,
                    0,
                    rue_core::spin::Spin::None,
                ),
                path: Arc::new(vec![]),
            },
            Node {
                game: empty_game(vec![Piece::T]),
                score: 50.0,
                root_move: Move::new(
                    Piece::T,
                    rue_core::rotation::Rotation::North,
                    1,
                    0,
                    rue_core::spin::Spin::None,
                ),
                path: Arc::new(vec![]),
            },
            Node {
                game: empty_game(vec![Piece::T]),
                score: 10.0,
                root_move: Move::new(
                    Piece::T,
                    rue_core::rotation::Rotation::North,
                    2,
                    0,
                    rue_core::spin::Spin::None,
                ),
                path: Arc::new(vec![]),
            },
        ];

        let config = SearchConfig {
            futility_delta: 60.0,
            beam_width: 10,
            ..SearchConfig::default()
        };
        sort_prune_truncate(&mut nodes, &config);

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
            garbage_row: 0,
            hold: None,
            queue: vec![Piece::I],
            garbage_queue: vec![],
            b2b_count: None,
            combo_count: None,
            ruleset: SEASON_2,
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

        let result = beam_search(&game, &config, &weights);
        assert!(result.is_some());
        let res = result.unwrap();

        assert!(
            !res.root_scores.is_empty(),
            "should have at least one root score"
        );
        assert_eq!(
            res.root_scores[0].0, res.best.root_move,
            "top root_score must match best root_move"
        );
    }
}
