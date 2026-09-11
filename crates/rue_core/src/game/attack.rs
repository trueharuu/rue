use crate::game::ruleset::Ruleset;
use crate::spin::Spin;

/// The chaining bonus applied for rulesets where [`Ruleset::b2b_chaining`] is
/// true.
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

/// Computes the attack for a clear from the scalar inputs.
///
/// This is the pure math shared by [`crate::game::Game::advance`] and the
/// beam search. `b2b` and `combo` are the values after the clear; `pre_b2b`
/// and `pre_combo` are the values before it.
#[must_use]
pub fn compute_attack(
    ruleset: &Ruleset,
    spin: Spin,
    line_clears: u32,
    is_pc: bool,
    b2b: Option<u32>,
    combo: Option<u32>,
    pre_b2b: Option<u32>,
    pre_combo: Option<u32>,
    canceled: u32,
) -> Attack {
    let clear_type = match line_clears {
        1 => Clear::Single,
        2 => Clear::Double,
        3 => Clear::Triple,
        4 => Clear::Quad,
        5 => Clear::Penta,
        _ => Clear::None,
    };

    let is_special_clear = spin != Spin::None || line_clears >= 4;
    let chain_broken = !(is_special_clear || is_pc && ruleset.pc_b2b.is_some());

    if line_clears == 0 {
        return Attack {
            clear_type,
            spin_type: spin,
            surge_release: None,
            line_clears: 0,
            is_perfect_clear: false,
            base_attack: 0,
            total: 0,
            canceled,
            b2b_count: pre_b2b,
            combo_count: pre_combo,
            chain_broken,
        };
    }

    let mut garbage = f64::from(ruleset.base_attack(line_clears, spin));

    if let Some(s) = b2b
        && s > 0
    {
        if ruleset.b2b_chaining {
            garbage += b2b_chaining_bonus(s, ruleset);
        } else {
            garbage += f64::from(ruleset.back_to_back_bonus);
        }
    }

    if let Some(combo) = combo
        && combo > 0
    {
        garbage *= 1.0 + ruleset.combo_bonus * f64::from(combo);
        if combo > 1 {
            let combo_floor = (1.0 + f64::from(combo) * ruleset.combo_floor_scale).ln();
            garbage = garbage.max(combo_floor);
        }
    }

    let main_event = (garbage * ruleset.garbage_multiplier).floor();
    let surge_event = if ruleset.b2b_charging {
        pre_b2b
            .filter(|_| chain_broken)
            .filter(|b| *b + 1 > ruleset.b2b_charging_start)
            .map_or(0.0, |b| {
                (f64::from(b - ruleset.b2b_charging_start + ruleset.back_to_back_bonus + 1)
                    * ruleset.garbage_multiplier)
                    .floor()
                    .max(0.0)
            })
    } else {
        0.0
    };
    let pc_event = if is_pc {
        (f64::from(ruleset.pc_garbage) * ruleset.garbage_multiplier).floor()
    } else {
        0.0
    };

    let is_surge_release = surge_event > 0.0;

    Attack {
        clear_type,
        spin_type: spin,
        surge_release: if is_surge_release {
            Some(surge_event as u32)
        } else {
            None
        },
        line_clears,
        is_perfect_clear: is_pc,
        base_attack: ruleset.base_attack(line_clears, spin),
        total: (main_event + surge_event + pc_event) as u32,
        canceled,
        b2b_count: pre_b2b,
        combo_count: pre_combo,
        chain_broken,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clear {
    None,
    Single,
    Double,
    Triple,
    Quad,
    Penta,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attack {
    pub clear_type: Clear,
    pub spin_type: Spin,
    pub surge_release: Option<u32>,
    pub line_clears: u32,
    pub is_perfect_clear: bool,
    pub base_attack: u32,
    pub total: u32,
    pub canceled: u32,
    pub b2b_count: Option<u32>,
    pub combo_count: Option<u32>,
    pub chain_broken: bool,
}

impl Attack {
    #[must_use]
    pub fn total(&self) -> u32 {
        self.total
    }

    #[must_use]
    pub fn outgoing(&self) -> u32 {
        self.total - self.canceled
    }

    #[must_use]
    pub fn is_special_clear(&self) -> bool {
        self.line_clears > 0 && (self.spin_type != Spin::None || self.line_clears >= 4)
    }
}
