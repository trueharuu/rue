//! Recursive tree search traversal logic.
//!
//! Handles both leaf evaluation ([`count_locks`]) and inner node expansion,
//! with dynamic band-width casting to minimize memory per recursion level.

use rue_core::{board::Board, piece::Piece, spin::Spins};
use rue_nav::movegen::{count_locks, generate};

use crate::fusion::last_dispatch;
use crate::height::{band_words, height_after_clear_free};

use super::dispatch::perft_rec;

/// Leaf-level evaluation: count unreachable lockable placements.
///
/// At depth 1, we've reached the leaves and simply count the total reachable
/// positions for the final piece.
#[inline]
pub fn leaf<const P: Piece, const N: usize>(b: &Board<8>, h: i32) -> u64 {
    let b1: Board<N> = b.cast();
    u64::from(count_locks::<P, N>(&b1, h, 0))
}

/// Apply one lock for piece `P`, cast to width `M`, and recurse to the next depth.
///
/// The post-lock height is computed cheaply when no line clears occur, and
/// validated against the board-derived height in debug builds.
pub fn step_cast<const P: Piece, const N: usize, const M: usize>(
    b1: &Board<N>,
    rc: usize,
    x: i32,
    y: i32,
    q: &[Piece],
    depth: usize,
    h: i32,
) -> u64 {
    let mut b2: Board<M> = b1.cast();
    let clears = b2.do_move_masked(P, rc, x, y);

    let h2 = if clears == 0 {
        height_after_clear_free::<P>(y, rc, h)
    } else {
        b2.max_y()
    };

    debug_assert_eq!(h2, b2.max_y());
    let nb: Board<8> = b2.cast();
    perft_rec(&nb, q, depth, h2)
}

/// Expand an interior perft node by generating all lockable placements for `P`.
///
/// Uses dynamic band-width dispatch so recursive calls use the smallest
/// practical board word count for the estimated follow-up height.
pub fn inner<const P: Piece, const N: usize>(
    b: &Board<8>,
    q: &[Piece],
    depth: usize,
    h: i32,
) -> u64 {
    let b1: Board<N> = b.cast();
    let ml = generate::<P, { Spins::None }, N>(&b1, h, 0);
    let h2w = band_words(h + P.h_place());
    let rest = &q[1..];

    if depth == 2 {
        let p2 = rest[0];
        return if h2w == N {
            last_dispatch::<N, N>(&b1, &ml, P, p2, h)
        } else {
            match h2w {
                2 => last_dispatch::<N, 2>(&b1, &ml, P, p2, h),
                3 => last_dispatch::<N, 3>(&b1, &ml, P, p2, h),
                4 => last_dispatch::<N, 4>(&b1, &ml, P, p2, h),
                _ => last_dispatch::<N, 8>(&b1, &ml, P, p2, h),
            }
        };
    }

    let mut nodes = 0u64;
    let cs = P.canonical_rotations();
    let mut rc = 0;
    while rc < cs {
        ml.none[rc].for_each_set_bit(|x, y| {
            nodes += if h2w == N {
                step_cast::<P, N, N>(&b1, rc, x, y, rest, depth - 1, h)
            } else {
                match h2w {
                    2 => step_cast::<P, N, 2>(&b1, rc, x, y, rest, depth - 1, h),
                    3 => step_cast::<P, N, 3>(&b1, rc, x, y, rest, depth - 1, h),
                    4 => step_cast::<P, N, 4>(&b1, rc, x, y, rest, depth - 1, h),
                    _ => step_cast::<P, N, 8>(&b1, rc, x, y, rest, depth - 1, h),
                }
            };
        });
        rc += 1;
    }
    nodes
}

/// Dispatch traversal for a concrete next piece `P` and recursion depth.
///
/// This selects an appropriate board band width for either leaf counting or
/// inner expansion based on the current stack height and piece profile.
pub fn with_piece<const P: Piece>(b: &Board<8>, q: &[Piece], depth: usize, h: i32) -> u64 {
    debug_assert_eq!(h, b.max_y());
    let h1w = band_words(h + P.h_gen());

    if depth == 1 {
        return match h1w {
            1 => leaf::<P, 1>(b, h),
            2 => leaf::<P, 2>(b, h),
            3 => leaf::<P, 3>(b, h),
            4 => leaf::<P, 4>(b, h),
            _ => leaf::<P, 8>(b, h),
        };
    }

    match h1w {
        1 => inner::<P, 1>(b, q, depth, h),
        2 => inner::<P, 2>(b, q, depth, h),
        3 => inner::<P, 3>(b, q, depth, h),
        4 => inner::<P, 4>(b, q, depth, h),
        _ => inner::<P, 8>(b, q, depth, h),
    }
}
