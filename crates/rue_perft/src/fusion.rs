//! Fused final-level optimization.
//!
//! When depth == 2, the next ply consists entirely of leaf nodes. This module
//! fuses the iteration and leaf evaluation to avoid per-child dispatch overhead,
//! height rescans, and the full piece/band-width dispatch chain.

use rue_core::{board::Board, header::top_extent, piece::Piece};
use rue_nav::{buffer::Moves, movegen::count_locks};

use crate::height::band_words;

/// Fused final-level evaluation: iterate moves and count leaves.
///
/// For each move (x, y, rc) by piece P2, we place it (casting to band width M
/// as needed), compute its exact height, and count the unreachable lockable
/// placements for the final piece P2.
#[must_use]
pub fn last_level<const P2: Piece, const N: usize, const M: usize>(
    b1: &Board<N>,
    ml: &Moves<N>,
    p: Piece,
    h: i32,
) -> u64 {
    let mut nodes = 0u64;
    let cs = p.canonical_rotations();
    let mut rc = 0;

    while rc < cs {
        let te = top_extent(p, rc);
        ml.none[rc].for_each_set_bit(|x, y| {
            let mut b2: Board<M> = b1.cast();
            let clears = b2.do_move_masked(p, rc, x, y);

            let h2 = if clears == 0 {
                let t = y + te;
                if t > h { t } else { h }
            } else {
                b2.max_y()
            };

            debug_assert_eq!(h2, b2.max_y());
            nodes += u64::from(match band_words(h2 + P2.h_gen()) {
                1 => count_locks::<P2, 1>(&b2.cast(), h2, 0),
                2 => count_locks::<P2, 2>(&b2.cast(), h2, 0),
                3 => count_locks::<P2, 3>(&b2.cast(), h2, 0),
                4 => count_locks::<P2, 4>(&b2.cast(), h2, 0),
                _ => count_locks::<P2, 8>(&b2.cast(), h2, 0),
            });
        });
        rc += 1;
    }
    nodes
}

/// Dispatch the final piece type (P2) for [`last_level`].
#[inline]
#[must_use]
pub fn last_level_dispatch_inner<const N: usize, const M: usize>(
    b1: &Board<N>,
    ml: &Moves<N>,
    p: Piece,
    p2: Piece,
    h: i32,
) -> u64 {
    match p2 {
        Piece::I => last_level::<{ Piece::I }, N, M>(b1, ml, p, h),
        Piece::O => last_level::<{ Piece::O }, N, M>(b1, ml, p, h),
        Piece::T => last_level::<{ Piece::T }, N, M>(b1, ml, p, h),
        Piece::J => last_level::<{ Piece::J }, N, M>(b1, ml, p, h),
        Piece::L => last_level::<{ Piece::L }, N, M>(b1, ml, p, h),
        Piece::S => last_level::<{ Piece::S }, N, M>(b1, ml, p, h),
        Piece::Z => last_level::<{ Piece::Z }, N, M>(b1, ml, p, h),
    }
}

/// Dispatch the next band width for [`last_level`].
#[must_use]
pub fn last_dispatch<const N: usize, const M: usize>(
    b1: &Board<N>,
    ml: &Moves<N>,
    p: Piece,
    p2: Piece,
    h: i32,
) -> u64 {
    last_level_dispatch_inner::<N, M>(b1, ml, p, p2, h)
}

/// Re-export for traversal module to dispatch on M.
#[must_use]
pub fn last_level_dispatch<const N: usize>(
    b1: &Board<N>,
    ml: &Moves<N>,
    p: Piece,
    p2: Piece,
    h: i32,
    h2w: usize,
) -> u64 {
    if h2w == N {
        last_dispatch::<N, N>(b1, ml, p, p2, h)
    } else {
        match h2w {
            2 => last_dispatch::<N, 2>(b1, ml, p, p2, h),
            3 => last_dispatch::<N, 3>(b1, ml, p, p2, h),
            4 => last_dispatch::<N, 4>(b1, ml, p, p2, h),
            _ => last_dispatch::<N, 8>(b1, ml, p, p2, h),
        }
    }
}
