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
use rue_eval::simple::Simple;

/// Pieces per second.
pub const PPS: f64 = 300.0;
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
    let mut total_attack = 0u32;
    let mut pieces = 0u32;
    let mut chain_pieces = 0u32;
    let mut chain_b2b = 0u32;
    let i_total = Instant::now();
    loop {
        let i = Instant::now();
        let best = best_placement(&game);
        let e = i.elapsed();
        if best.is_none() {
            println!("dead");
            break;
        }

        let (best, score) = best.unwrap();
        println!("{}", render_with(game.board, &best));
        let h = game.hold.map_or_else(String::new, |x| x.to_string());
        let q = game.queue[..6]
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<String>();
        let out = game.tick(best);
        if game.b2b_count.is_none() {
            chain_pieces = 0;
            chain_b2b = 0;
        } else {
            chain_pieces += 1;
            if out.is_b2b {
                chain_b2b += 1;
            }
        }
        pieces += 1;
        total_attack += out.outgoing as u32;
        println!(
            "{score:.3} {e:.2?} [{h}]{q} sent {}/{}",
            out.outgoing, out.line_clears
        );
        println!("choosable active pieces: {:?}", game.active());
        println!(
            "b2b={:?} combo={:?} pieces/second={:.3} attack/piece={:.3} b2b/bag={:.3}",
            game.b2b_count,
            game.combo_count,
            f64::from(pieces) / i_total.elapsed().as_secs_f64(),
            f64::from(total_attack) / f64::from(pieces),
            f64::from(chain_b2b) / (f64::from(chain_pieces) / 7.0)
        );
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
use rue_search::{SearchConfig, beam_search};

/// The best placement at any given time.
#[must_use]
pub fn best_placement<const N: usize>(game: &Game<N>) -> Option<(Move, f64)> {
    let model = Simple {
        b2b: 5.0,
        holes: -4.0,
        cell_coveredness: -4.5,
        height: -0.2,
        height_half: -1.0,
        height_three_quarters: -5.0,
        bumpiness: -0.3,
        bumpiness_sq: -0.1,
        row_transitions: -0.3,
        active: [
            [0.0, 3.5, 3.5],
            [-1.0, 2.0, 1.0],
            [-1.0, 0.5, 2.0],
            [-1.0, 0.5, 2.0],
            [2.0, 0.0, 0.0],
        ],
        combo: 0.5,
        sent: 0.5,
        well_col: [-0.5, -1.0, 0.2, 2.0, 1.0, 1.0, 2.0, 0.2, -1.0, -0.5],
        well_depth: 1.0,
    };

    let cfg = SearchConfig {
        beam_width: 500,
        depth: 7,
        futility_delta: 0.0,
    };

    let result = beam_search(game, &cfg, &model);
    result.map(|x| (x.best.root_move, x.best.score))
}
