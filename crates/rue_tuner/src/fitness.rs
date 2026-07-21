//! Fitness evaluation for tuning.
//!
//! Runs self-play games using a given set of weights and returns mean
//! back-to-back per bag as a scalar fitness score.

use rayon::iter::{ParallelBridge, ParallelIterator};
use rue_core::board::Board;
use rue_core::game::Game;
use rue_core::game::garbage::GarbageQueue;
use rue_core::game::ruleset::SEASON_2;
use rue_core::rng::{Rng, RngKind};
use rue_eval::weights::Weights;
use rue_search::beam_search;

use crate::config::FitnessConfig;

/// Fill the queue with `n` bags of 7-bag random pieces.
fn fill(queue: &mut Vec<rue_core::piece::Piece>, rng: &mut Rng, n: usize) {
    for _ in 0..n {
        let mut slice: Vec<rue_core::piece::Piece> = RngKind::Bag7.slice();
        rng.shuffle_array(&mut slice);
        queue.extend_from_slice(&slice);
    }
}

/// Run a single self-play game and return the mean back-to-back per bag.
///
/// The game plays `config.pieces` placements using beam search with the
/// given `weights`. Returns `0.0` if the board tops out immediately.
pub fn single_game<const N: usize, W: Weights>(
    weights: &W,
    config: &FitnessConfig,
    seed: i32,
) -> f64 {
    let cfg = config.search_config();

    let mut game = Game {
        board: Board::<N>::EMPTY,
        hold: None,
        queue: vec![],
        garbage_queue: GarbageQueue::new(),
        b2b_count: None,
        combo_count: None,
        ruleset: SEASON_2,
        rng: Rng::new_seeded(seed),
    };

    fill(&mut game.queue, &mut game.rng, 3);

    let mut total_b2b = 0.0_f64;
    let mut pieces = 0_usize;

    while pieces < config.pieces {
        let best = beam_search(&game, &cfg, weights);
        let Some(result) = best else {
            // immediately increment pieces to the max with 0 additional b2b
            pieces = config.pieces;
            break;
        };

        let out = game.tick(result.best.root_move);
        total_b2b += f64::from(out.is_b2b());
        pieces += 1;

        if game.queue.len() <= 14 {
            fill(&mut game.queue, &mut game.rng, 2);
        }
    }

    println!("game {seed}, fitness = {}", total_b2b / pieces as f64);
    if pieces == 0 {
        0.0
    } else {
        total_b2b / pieces as f64
    }
}

/// Average [`single_game`] over `config.games` independent seeds.
///
/// Returns the mean back-to-back per bag across all games.
pub fn multi_game<const N: usize, W: Weights>(weights: &W, config: &FitnessConfig) -> f64 {
    let total: f64 = (0..config.games)
        .par_bridge()
        .map(|i| single_game::<N, W>(weights, config, i as i32 + 1))
        .sum();
    total / config.games as f64
}
