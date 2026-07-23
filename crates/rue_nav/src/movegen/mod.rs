//! Move-generation internals split into low-level ops and search.

use rue_core::board::Board;
use rue_core::game::ruleset::Ruleset;
use rue_core::piece::Piece;
use rue_core::spin::Spins;

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
pub fn generate_inlined<const P: Piece, const SPINS: Spins, const N: usize>(
    b: &Board<N>,
    y: i32,
    force: i32,
    sonic: bool,
) -> Moves<N> {
    gen_impl::<P, SPINS, N, true>(b, y, force, sonic).0
}

/// Generate all landable placements for a piece on the given board.
/// Forwards the arguments as const generics to `generate_inlined`.
///
/// Returns a `Moves` structure with placements categorized by spin type (None, Mini, Full).
/// When `SPINS = Spins::None`, all placements are in the `none` array.
#[inline]
#[must_use]
pub fn generate<const N: usize>(
    b: &Board<N>,
    ruleset: Ruleset,
    p: Piece,
    y: i32,
    force: i32,
) -> Moves<N> {
    // TODO: simplify massive match statement with 28 branches
    match (p, ruleset.spins) {
        (Piece::T, Spins::None) => {
            generate_inlined::<{ Piece::T }, { Spins::None }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::I, Spins::None) => {
            generate_inlined::<{ Piece::I }, { Spins::None }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::J, Spins::None) => {
            generate_inlined::<{ Piece::J }, { Spins::None }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::L, Spins::None) => {
            generate_inlined::<{ Piece::L }, { Spins::None }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::O, Spins::None) => {
            generate_inlined::<{ Piece::O }, { Spins::None }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::S, Spins::None) => {
            generate_inlined::<{ Piece::S }, { Spins::None }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::Z, Spins::None) => {
            generate_inlined::<{ Piece::Z }, { Spins::None }, _>(b, y, force, ruleset.inf_sdf)
        }

        (Piece::T, Spins::T) => generate_inlined::<{ Piece::T }, { Spins::T }, _>(b, y, force, ruleset.inf_sdf),
        (Piece::I, Spins::T) => generate_inlined::<{ Piece::I }, { Spins::T }, _>(b, y, force, ruleset.inf_sdf),
        (Piece::J, Spins::T) => generate_inlined::<{ Piece::J }, { Spins::T }, _>(b, y, force, ruleset.inf_sdf),
        (Piece::L, Spins::T) => generate_inlined::<{ Piece::L }, { Spins::T }, _>(b, y, force, ruleset.inf_sdf),
        (Piece::O, Spins::T) => generate_inlined::<{ Piece::O }, { Spins::T }, _>(b, y, force, ruleset.inf_sdf),
        (Piece::S, Spins::T) => generate_inlined::<{ Piece::S }, { Spins::T }, _>(b, y, force, ruleset.inf_sdf),
        (Piece::Z, Spins::T) => generate_inlined::<{ Piece::Z }, { Spins::T }, _>(b, y, force, ruleset.inf_sdf),

        (Piece::T, Spins::AllMini) => {
            generate_inlined::<{ Piece::T }, { Spins::AllMini }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::I, Spins::AllMini) => {
            generate_inlined::<{ Piece::I }, { Spins::AllMini }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::J, Spins::AllMini) => {
            generate_inlined::<{ Piece::J }, { Spins::AllMini }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::L, Spins::AllMini) => {
            generate_inlined::<{ Piece::L }, { Spins::AllMini }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::O, Spins::AllMini) => {
            generate_inlined::<{ Piece::O }, { Spins::AllMini }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::S, Spins::AllMini) => {
            generate_inlined::<{ Piece::S }, { Spins::AllMini }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::Z, Spins::AllMini) => {
            generate_inlined::<{ Piece::Z }, { Spins::AllMini }, _>(b, y, force, ruleset.inf_sdf)
        }

        (Piece::T, Spins::AllPlus) => {
            generate_inlined::<{ Piece::T }, { Spins::AllPlus }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::I, Spins::AllPlus) => {
            generate_inlined::<{ Piece::I }, { Spins::AllPlus }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::J, Spins::AllPlus) => {
            generate_inlined::<{ Piece::J }, { Spins::AllPlus }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::L, Spins::AllPlus) => {
            generate_inlined::<{ Piece::L }, { Spins::AllPlus }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::O, Spins::AllPlus) => {
            generate_inlined::<{ Piece::O }, { Spins::AllPlus }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::S, Spins::AllPlus) => {
            generate_inlined::<{ Piece::S }, { Spins::AllPlus }, _>(b, y, force, ruleset.inf_sdf)
        }
        (Piece::Z, Spins::AllPlus) => {
            generate_inlined::<{ Piece::Z }, { Spins::AllPlus }, _>(b, y, force, ruleset.inf_sdf)
        }
    }
}

/// Count the number placements that could be reached.
///
/// Does not classify placements by spin type, nor does it return the placements themselves.
#[inline]
#[must_use]
pub fn count_locks<const P: Piece, const N: usize>(b: &Board<N>, y: i32, force: i32, sonic: bool) -> u32 {
    gen_impl::<P, { Spins::None }, N, false>(b, y, force, sonic).1
}
