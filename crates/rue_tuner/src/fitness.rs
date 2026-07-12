//! Self-play and static fitness evaluation for weight optimization.

// use std::time::Instant;

use rue_core::{
    board::Board,
    game::{Game, garbage::GarbageQueue, ruleset::SEASON_2},
    piece::Piece,
    rng::{Rng, RngKind},
};
use rue_eval::weights::Weights;
use rue_search::{SearchConfig, beam_search};

/// Play a single game to completion, returning attack per piece.
///
/// Uses a lightweight search config for speed during tuning iterations.
pub fn play_one_game<W: Weights + Sync>(weights: &W, depth: usize, beam_width: usize, max_n: usize) -> f64 {
    let mut game = Game::<8> {
        board: Board::<8>::EMPTY,
        hold: None,
        queue: Vec::new(),
        garbage_queue: GarbageQueue::new(),
        b2b_count: None,
        combo_count: None,
        ruleset: SEASON_2,
        rng: Rng::new(),
    };

    fill_queue(&mut game.queue, &mut game.rng, 2);
    let cfg = SearchConfig {
        beam_width,
        depth,
        futility_delta: 0.0,
        time_budget_ms: Some(50),
        ..SearchConfig::default()
    };

    let mut pieces = 0u32;
    let mut total_attack = 0.0f64;
    loop {
        if pieces >= max_n as u32 {
            break;
        }

        // let i = Instant::now();
        let result = beam_search(&game, &cfg, weights);
        // let elapsed = i.elapsed();
        // println!("[tuner] search time ({pieces}): {elapsed:.3?}");
        let Some(result) = result else {
            break;
        };

        let mv = result.best.root_move;
        let ctx = game.tick(mv);
        total_attack += f64::from(ctx.attack_sent);
        pieces += 1;

        if game.queue.len() <= 14 {
            fill_queue(&mut game.queue, &mut game.rng, 2);
        }
    }

    if pieces == 0 {
        0.0
    } else {
        total_attack / f64::from(pieces)
    }
}

/// Run `n` self-play games and return the mean attack per piece.
pub fn self_play_fitness<W: Weights + Sync>(
    weights: &W,
    n: usize,
    depth: usize,
    beam_width: usize,
    max_n: usize,
) -> f64 {
    let total: f64 = (0..n)
        .map(|_| play_one_game(weights, depth, beam_width, max_n))
        .sum();
    total / n as f64
}

/// Append `n` bags (7*n pieces) to the queue.
fn fill_queue(p: &mut Vec<Piece>, r: &mut Rng, n: usize) {
    for _ in 0..n {
        let mut slice: Vec<Piece> = RngKind::Bag7.slice();
        r.shuffle_array(&mut slice);
        p.extend_from_slice(&slice);
    }
}
