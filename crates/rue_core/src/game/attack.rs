//! Utilities for attack calculation.
use crate::game::ruleset::Ruleset;
use crate::piece::Piece;
use crate::placement::Move;
use crate::spin::Spin;

/// The chaining bonus applied for rulesets where [`Ruleset::b2b_chaining`] is true.
#[must_use]
pub fn b2b_chaining_bonus(b2b: u32, ruleset: &Ruleset) -> f64 {
    if b2b <= 1 {
        return f64::from(ruleset.back_to_back_bonus);
    }

    let log_part = (1.0 + f64::from(b2b) * ruleset.b2b_chaining_log).ln();
    let floored = (1.0 + log_part).floor();

    let remainder = (1.0 + log_part) - floored;
    let third = if remainder > 0.0 {
        remainder / 3.0
    } else {
        0.0
    };
    floored + third
}

/// The amount of lines cleared.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Clear {
    None,
    Single,
    Double,
    Triple,
    Quad,
    Penta,
}

impl Clear {
    /// Returns the number of lines cleared.
    #[inline]
    #[must_use]
    pub const fn count(&self) -> u8 {
        match self {
            Clear::None => 0,
            Clear::Single => 1,
            Clear::Double => 2,
            Clear::Triple => 3,
            Clear::Quad => 4,
            Clear::Penta => 5,
        }
    }
}

/// Full statistics for an attack.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackContext {
    pub clear_type: Clear,
    pub spin_type: Spin,
    pub lines_cleared: u8,
    pub attack_sent: f32,
    pub b2b_before: u8,
    pub b2b_after: u8,
    pub combo_before: u32,
    pub combo_after: u32,
    pub is_surge_release: bool,
    pub is_garbage_clear: bool,
    pub is_perfect_clear: bool,
    pub piece: Piece,
    pub placement: Move,
}

impl AttackContext {
    /// Returns true if this attack continues a back-to-back chain.
    #[inline]
    #[must_use]
    pub const fn is_b2b(&self) -> bool {
        self.b2b_after > self.b2b_before
    }
}
