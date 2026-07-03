//! Move-generation internals split into low-level ops and search.

use rue_core::{board::Board, piece::Piece, spin::Spins};

use crate::{buffer::Moves, movegen::search::gen_impl};

pub mod op;
pub mod search;

/// Generate all landable placements for a piece on the given board.
///
/// Returns a `Moves` structure with placements categorized by spin type (None, Mini, Full).
/// When `SPINS = Spins::None`, all placements are in the `none` array.
#[inline]
#[must_use]
pub fn generate<const P: Piece, const SPINS: Spins, const N: usize>(
    b: &Board<N>,
    y: i32,
    force: i32,
) -> Moves<N> {
    gen_impl::<P, SPINS, N, true>(b, y, force).0
}
 
/// Count the number of lockable placements that could not be reached.
///
/// Used to evaluate perfect clear feasibility and search pruning.
/// Only populated when `SPINS = Spins::None` (spin detection disables counting).
#[inline]
#[must_use]
pub fn count_locks<const P: Piece, const N: usize>(
    b: &Board<N>,
    y: i32,
    force: i32,
) -> u32 {
    gen_impl::<P, { Spins::None }, N, false>(b, y, force).1
}
