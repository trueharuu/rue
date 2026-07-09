//! Utilities for attack calculation.
use crate::{game::ruleset::Ruleset, placement::Move};

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

/// Full statistics for an attack.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackContext {
    /// The piece placed.
    pub placement: Move,
    /// Total number of line clears performed.
    pub line_clears: usize,
    /// Whether the resulting board is a Perfect Clear.
    pub is_pc: bool,
    /// Total number of lines sent, prior to garbage calculations.
    pub outgoing: f64,
    /// Whether this is a B2B clear.
    pub is_b2b: bool,
    /// Total garbage canceled by the clear.
    pub garbage_cancelled: f64,
    /// Total garbage tanked by this placement. Mututally exclusive with `garbage_cancelled` and `sent`.
    pub garbage_tanked: f64,
    /// Total number of lines sent *to the opponent*.
    pub sent: f64,
}