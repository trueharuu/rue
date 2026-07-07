//! Utilities for attack calculation.
use crate::game::ruleset::Ruleset;

/// The chaining bonus applied for rulesets where [`Ruleset::b2b_chaining`] is true.
pub fn b2b_chaining_bonus(b2b: u32, ruleset: &Ruleset) -> f64 {
    if b2b <= 1 {
        return ruleset.back_to_back_bonus as f64;
    }

    let log_part = (1.0 + b2b as f64 * ruleset.b2b_chaining_log).ln();
    let floored = (1.0 + log_part).floor();
    
    let remainder = (1.0 + log_part) - floored;
    let third = if remainder > 0.0 {
        remainder / 3.0
    } else {
        0.0
    };
    floored + third
}
