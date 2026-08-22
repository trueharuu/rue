//! Spin classification enums used by placement metadata.

use std::marker::ConstParamTy;

/// Spin outcome classification for a concrete [`crate::placement::Move`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, ConstParamTy)]
pub enum Spin {
    /// Not a spin.
    None = 0,
    /// Mini spin.
    Mini = 1,
    /// Full spin.
    Full = 2,
}

impl Spin {
    /// Number of spin outcome variants.
    pub const NB: usize = 3;
    /// All spin outcomes in index order.
    pub const ALL: [Spin; Self::NB] = [Spin::None, Spin::Mini, Spin::Full];
}

/// Spin allowance policy for move generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ConstParamTy)]
pub enum Spins {
    /// Disable spins entirely.
    None = 0,
    /// Only allow T-spins via 3-corner detection.
    T = 1,
    /// Only allow T-spins via 3-corner detection, plus immobile spins for T pieces marked as [`Spin::Mini`].
    TPlus = 2,
    /// [`Spins::T`], plus immobile spins for all non-T pieces, marked as [`Spin::Mini`].
    AllMini = 3,
    /// [`Spins::TPlus`], plus immobile spins for all non-T pieces, marked as [`Spin::Mini`].
    AllMiniPlus = 4,
    /// [`Spins::T`], plus immobile spins for all non-T pieces, marked as [`Spin::Full`].
    All = 5,
    /// [`Spins::TPlus`], plus immobile spins for all non-T pieces, marked as [`Spin::Full`].
    AllPlus = 6,
    /// All placements reached via rotation are a spin, marked as [`Spin::Full`].
    Stupid = 7,
}
