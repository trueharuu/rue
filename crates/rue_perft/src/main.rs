//! Binary entrypoint for the [`rue_nav`] performance test.

use rue_core::board::Board;
use rue_core::game::ruleset::Handling;
use rue_core::piece::Piece;
use rue_core::placement::Move;
use rue_core::rotation::Rotation;
use rue_core::spin::Spin;
use rue_core::spin::Spins;
use rue_nav::pathfinder;
use rue_perft::height::parse_queue;
use rue_perft::perft_mt;
use std::time::Instant;

/// Entry point.
pub fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        println!("Usage: chimera_perft <queue>");
        return;
    }

    let queue = parse_queue(&args[0]).expect("Invalid queue");
    let i = Instant::now();
    let result = perft_mt(&queue);
    let elapsed = i.elapsed();
    println!(
        "perft({queue:?}) = {result} in {elapsed:?} ({} nodes/sec)",
        human(result as f64 / elapsed.as_secs_f64())
    );

    #[allow(clippy::items_after_statements)]
    const P: Piece = Piece::S;
    let mut b = Board::<2>::EMPTY;
    b.set_many(&[
        (0, 0),
        (0, 1),
        (0, 2),
        (1, 0),
        (2, 2),
        (3, 0),
        (3, 1),
        (3, 2),
        (3, 3),
    ]);

    #[allow(clippy::items_after_statements)]
    const RULE: Handling = Handling {
        inf_sdf: true,
        spins: Spins::AllMini,
        srs_plus: false,
        use_180: true,
        finesse: true,
    };



    let _ = pathfinder::get_input::<8, { RULE }>(
        &b.cast(),
        Move::new(P, Rotation::West, 2, 1, Spin::Mini),
    );
    // let mv = Move::new(P, Rotation::West, 2, 1, Spin::Mini);
    // let mv2 = mv.canonicalize();
    // println!("{}", render_with(b, &mv));
    // println!("{}", render_with(b, &mv2));
    
    // let m = movegen::generate_inlined::<{ P }, { RULE }, 2>(&b, 20, 0);
    // for p in m.iter() {
    //     // println!("{p:?}");
    //     println!("{}", render_with(b, &p));
    //     assert!(!path.is_empty(), "failed to get finesse for: {p:?}");
    //     println!("{path:?}");
    // }

    // // mini
    // println!(
    //     "{}",
    //     render::merge(
    //         b,
    //         m.has3[1] & m.via_rotation[1] & m.landed[1] & !m.front2[1] & !m.via_5th_kick[1]
    //     )
    // );
    // // mini, upgraded to full because 5th kick
    // println!(
    //     "{}",
    //     render::merge(
    //         b,
    //         m.has3[1] & m.via_rotation[1] & m.landed[1] & !m.front2[1] & m.via_5th_kick[1]
    //     )
    // );
    // // full
    // println!(
    //     "{}",
    //     render::merge(b, m.has3[1] & m.via_rotation[1] & m.landed[1] & m.front2[1])
    // );
    // // immobile. there are not spins in Spins::T/Spins::None, but are Spin::Mini in AllMini/AllPlus.
    // println!(
    //     "{}",
    //     render::merge(b, m.via_rotation[1] & m.landed[1] & m.immobile[1])
    // );
}

/// Format a number with K/M/B suffixes for thousands/millions/billions.
fn human(n: f64) -> String {
    let mut n = n;
    let mut suffix = "";
    if n >= 1_000_000_000.0 {
        n /= 1_000_000_000.0;
        suffix = "B";
    } else if n >= 1_000_000.0 {
        n /= 1_000_000.0;
        suffix = "M";
    } else if n >= 1_000.0 {
        n /= 1_000.0;
        suffix = "K";
    }

    format!("{n:.3}{suffix}")
}

#[cfg(test)]
mod tests {
    use rue_core::piece::Piece;
    use rue_perft::perft_mt;

    #[test]
    fn perft_i() {
        assert_eq!(perft_mt(&[Piece::I]), 17);
    }

    #[test]
    fn perft_io() {
        assert_eq!(perft_mt(&[Piece::I, Piece::O]), 153);
    }

    #[test]
    fn perft_iol() {
        assert_eq!(perft_mt(&[Piece::I, Piece::O, Piece::L]), 5266);
    }

    #[test]
    fn perft_iolj() {
        assert_eq!(perft_mt(&[Piece::I, Piece::O, Piece::L, Piece::J]), 188_374);
    }

    #[test]
    fn perft_ioljs() {
        assert_eq!(
            perft_mt(&[Piece::I, Piece::O, Piece::L, Piece::J, Piece::S]),
            3_497_187
        );
    }

    #[test]
    fn perft_iiiiii() {
        assert_eq!(perft_mt(&[Piece::I; 6]), 33_325_345);
    }

    #[test]
    fn perft_ioljsz() {
        assert_eq!(
            perft_mt(&[Piece::I, Piece::O, Piece::L, Piece::J, Piece::S, Piece::Z]),
            67_002_200
        );
    }

    #[test]
    fn perft_ioljszt() {
        assert_eq!(
            perft_mt(&[
                Piece::I,
                Piece::O,
                Piece::L,
                Piece::J,
                Piece::S,
                Piece::Z,
                Piece::T
            ]),
            2_647_076_135
        );
    }
}
