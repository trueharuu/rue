//! Move-generation internals split into low-level ops and search.

use rue_core::board::Board;
use rue_core::game::ruleset::Handling;
use rue_core::piece::Piece;

use crate::buffer::Moves;
use crate::movegen::search::gen_impl;

pub mod op;
pub mod search;

/// Generate all landable placements for a piece on the given board.
///
/// Returns a `Moves` structure with placements categorized by spin type (None, Mini, Full).
/// When `SPINS = Spins::None`, all placements are in the `none` array.
#[inline]
#[must_use]
pub fn generate_inlined<const P: Piece, const RULE: Handling, const N: usize>(
    b: &Board<N>,
    y: i32,
    force: i32,
) -> Moves<N> {
    gen_impl::<P, RULE, N, true>(b, y, force).0
}

/// Generate all landable placements for a piece on the given board.
/// Forwards the arguments as const generics to `generate_inlined`.
///
/// Returns a `Moves` structure with placements categorized by spin type (None, Mini, Full).
/// When `SPINS = Spins::None`, all placements are in the `none` array.
#[inline]
#[must_use]
pub fn generate<const N: usize, const RULE: Handling>(
    b: &Board<N>,
    p: Piece,
    y: i32,
    force: i32,
) -> Moves<N> {
    // TODO: simplify massive match statement with 28 branches
    match p {
        Piece::T => generate_inlined::<{ Piece::T }, { RULE }, _>(b, y, force),
        Piece::I => generate_inlined::<{ Piece::I }, { RULE }, _>(b, y, force),
        Piece::J => generate_inlined::<{ Piece::J }, { RULE }, _>(b, y, force),
        Piece::L => generate_inlined::<{ Piece::L }, { RULE }, _>(b, y, force),
        Piece::O => generate_inlined::<{ Piece::O }, { RULE }, _>(b, y, force),
        Piece::S => generate_inlined::<{ Piece::S }, { RULE }, _>(b, y, force),
        Piece::Z => generate_inlined::<{ Piece::Z }, { RULE }, _>(b, y, force),
    }
}

/// Count the number placements that could be reached.
///
/// Does not classify placements by spin type, nor does it return the placements themselves.
#[inline]
#[must_use]
pub fn count_locks<const P: Piece, const RULE: Handling, const N: usize>(
    b: &Board<N>,
    y: i32,
    force: i32,
) -> u32 {
    gen_impl::<P, { RULE }, N, false>(b, y, force).1
}
