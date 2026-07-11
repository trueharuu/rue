//! Spin classification enums used by placement metadata.

use std::marker::ConstParamTy;

/// T-spin outcome classification for a concrete move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Allow only T-spins.
    T = 1,
    /// Allow spins for all pieces, where non-T-spins are classified as [`Spin::Mini`].
    AllMini = 2,
    /// Allow spins for all pieces.
    AllPlus = 3,
}

impl Spins {
    /// Whether the policy allows immobility-based spin detection.
    #[must_use]
    pub const fn has_immobile(self) -> bool {
        matches!(self, Spins::AllMini | Spins::AllPlus)
    }

    /// Whether the policy utilizes 3-corner T-spin detection.
    #[must_use]
    pub const fn has_3corner(self) -> bool {
        matches!(self, Spins::T | Spins::AllPlus | Spins::AllMini)
    }
}