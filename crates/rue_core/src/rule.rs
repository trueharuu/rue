//! Gameplay rules governing move generation and placement.

use std::marker::ConstParamTy;

use crate::spin::Spins;

/// Ruleset parameters for move generation and placement logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ConstParamTy)]
pub struct Rule {
    /// Spin allowance policy used during move generation.
    pub spins: Spins,
    /// Enables infinite soft drop, lowering the piece as far as possible per input.
    pub inf_sdf: bool,
    /// Allows 180-degree rotations during move generation.
    pub allow_180: bool,
    /// Enables extended lateral movement along a row (DAS).
    pub das: bool,
    /// Spawn column for new pieces.
    pub spawn_x: i32,
    /// Spawn row for new pieces.
    pub spawn_y: i32,
}

/// The default [`Rule`] used by the sandbox and reference move generation.
pub const DEFAULT: Rule = Rule {
    spins: Spins::AllMini,
    inf_sdf: true,
    allow_180: true,
    das: true,
    spawn_x: 4,
    spawn_y: 21,
};

impl Rule {
    /// Returns `true` if the rule uses 3-corner detection for T-spin classification, `false` otherwise.
    #[inline]
    #[must_use]
    pub const fn has_t_corner_spins(&self) -> bool {
        !matches!(self.spins, Spins::None)
    }

    /// Returns `true` if the rule uses immobile detection for T-spin classification, `false` otherwise.
    #[inline]
    #[must_use]
    pub const fn has_immobile_t_spins(&self) -> bool {
        matches!(
            self.spins,
            Spins::TPlus | Spins::AllMiniPlus | Spins::AllPlus
        )
    }

    /// Returns `true` if the rule uses immobile detection for any non-T piece, `false` otherwise.
    #[inline]
    #[must_use]
    pub const fn has_immobile_non_t_spins(&self) -> bool {
        matches!(
            self.spins,
            Spins::AllMini | Spins::AllMiniPlus | Spins::All | Spins::AllPlus
        )
    }

    /// Returns `true` if spins are upgraded to full spins when immobile, `false` otherwise.
    #[inline]
    #[must_use]
    pub const fn is_full(&self) -> bool {
        matches!(self.spins, Spins::AllPlus | Spins::All)
    }
}
