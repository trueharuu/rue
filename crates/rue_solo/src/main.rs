//! Test crate for the singleplayer gameplay loop.

use std::time::Instant;

use rue_core::{
    board::Board,
    game::{Game, ruleset::SEASON_2},
    piece::Piece,
    placement::Move,
    render::render_with,
    rng::{Rng, RngKind},
};
use rue_eval::{simple::Simple, weights::Weights};
use rue_nav::buffer::Moves;

/// Pieces per second.
pub const PPS: f64 = 3.0;
/// Entry point.
pub fn main() {
    let mut rng = Rng::new();

    let mut game = Game {
        board: Board::<8>::EMPTY,
        garbage_row: 0,
        hold: None,
        queue: vec![],
        garbage_queue: vec![],
        b2b_count: None,
        combo_count: None,
        ruleset: SEASON_2,
    };

    fill(&mut game.queue, &mut rng, 3);

    loop {
        let i = Instant::now();
        let best = best_placement(&game);
        let e = i.elapsed();
        if best.is_none() {
            println!("dead");
            break;
        }

        let (best, score) = best.unwrap();
        println!("{}\n{score}", render_with(game.board, &best));
        game.tick(best);
        // break;
        if game.queue.len() <= 14 {
            fill(&mut game.queue, &mut rng, 2);
        }

        let sleep = (1.0 / PPS) - e.as_secs_f64();
        if sleep > 0.0 {
            std::thread::sleep(std::time::Duration::from_secs_f64(sleep));
        }
    }
}

/// Appends 14 pieces to the end of the queue.
fn fill(p: &mut Vec<Piece>, r: &mut Rng, n: usize) {
    for _ in 0..n {
        let mut slice: Vec<Piece> = RngKind::Bag7.slice();
        r.shuffle_array(&mut slice);
        p.extend_from_slice(&slice);
    }
}

/// The best placement at any given time.
pub fn best_placement<const N: usize>(game: &Game<N>) -> Option<(Move, f64)> {
    let placements_active =
        rue_nav::movegen::generate(&game.board, game.ruleset, game.active(), 20, 0);
    let placements_held = if let Some(h) = game.hold
        && h != game.active()
    {
        rue_nav::movegen::generate(&game.board, game.ruleset, h, 20, 0)
    } else {
        Moves::empty(game.active())
    };

    let model = Simple {
        holes: -4.0,
        cell_coveredness: -0.5,
        height: -0.2,
        height_half: -1.0,
        height_three_quarters: -5.0,
        bumpiness: -0.3,
        bumpiness_sq: -0.1,
    };

    let mut best: Option<(Move, f64)> = None;
    for z in placements_active.iter().chain(placements_held.iter()) {
        let mut after = game.clone();
        after.tick(z);

        let ev = model.evaluate(&after, &z);
        // println!(
        //     "{}\nsuggested placement scores {ev}",
        //     render_with(game.board, &z)
        // );
        if let Some(b) = best.as_mut() {
            if ev > b.1 {
                *b = (z, ev);
            }
        } else {
            best = Some((z, ev));
        }
    }

    best
}
