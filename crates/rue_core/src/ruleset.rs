use std::fmt::Debug;

use crate::spin::SpinType;

#[derive(Copy, Clone, Debug)]
pub struct Rules {
    pub enable_180: bool,
    pub enable_tspin: bool,
    pub enable_allspin: bool,
    pub srs_plus: bool,
    pub spawn_row: i32,
}

pub static ACTIVE_RULES: Rules = Rules {
    enable_180: true,
    enable_tspin: true,
    enable_allspin: false,
    srs_plus: true,
    spawn_row: 21,
};

// base attack table — no spin
pub const SINGLE: u8 = 0;
pub const DOUBLE: u8 = 1;
pub const TRIPLE: u8 = 2;
pub const QUAD: u8 = 4;
pub const PENTA: u8 = 5;

// allspin attack (any piece with spin, not just T)
pub const SPIN_MINI: u8 = 0;
pub const SPIN: u8 = 0;
pub const SPIN_MINI_SINGLE: u8 = 0;
pub const SPIN_SINGLE: u8 = 2;
pub const SPIN_MINI_DOUBLE: u8 = 1;
pub const SPIN_DOUBLE: u8 = 4;
pub const SPIN_MINI_TRIPLE: u8 = 2;
pub const SPIN_TRIPLE: u8 = 6;
pub const SPIN_MINI_QUAD: u8 = 4;
pub const SPIN_QUAD: u8 = 10;
pub const SPIN_PENTA: u8 = 12;

pub const BACK_TO_BACK_BONUS: u8 = 1;
const B2B_CHAINING_LOG: f32 = 0.8;
const COMBO_BONUS: f32 = 0.25;
const COMBO_MINIFIER_LOG: f32 = 1.25;
const B2B_CHARGE_AT: u8 = 4;
const B2B_CHARGE_BASE: u8 = 3;

const CLASSIC_COMBO_TABLE: [u8; 11] = [0, 1, 1, 2, 2, 3, 3, 4, 4, 4, 5];
const MODERN_COMBO_TABLE: [u8; 13] = [0, 1, 1, 2, 2, 2, 3, 3, 3, 3, 3, 3, 4];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComboTable {
    Multiplier,
    Classic,
    Modern,
    None,
}

#[derive(Clone, Copy)]
pub struct AttackConfig {
    pub pc_garbage: u8,
    pub pc_b2b: u8,
    pub b2b_chaining: bool,
    pub b2b_charging: bool,
    pub combo_table: ComboTable,
    pub garbage_multiplier: f32,
}

impl Debug for AttackConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AttackConfig")
    }
}

impl AttackConfig {
    pub fn tetra_league() -> Self {
        Self {
            pc_garbage: 5,
            pc_b2b: 2,
            b2b_chaining: false,
            b2b_charging: true,
            combo_table: ComboTable::Multiplier,
            garbage_multiplier: 1.0,
        }
    }

    pub fn season_one() -> Self {
        Self {
            pc_garbage: 10,
            pc_b2b: 0,
            b2b_chaining: true,
            b2b_charging: false,
            combo_table: ComboTable::Multiplier,
            garbage_multiplier: 1.0,
        }
    }

    pub fn quick_play() -> Self {
        Self {
            pc_garbage: 3,
            pc_b2b: 2,
            b2b_chaining: false,
            b2b_charging: true,
            combo_table: ComboTable::Multiplier,
            garbage_multiplier: 1.0,
        }
    }
}

/// base garbage for a line clear + spin type (piece-agnostic)
fn base_attack(lines: u8, spin: SpinType) -> f32 {
    match spin {
        SpinType::None => match lines {
            0 => 0.0,
            1 => SINGLE as f32,
            2 => DOUBLE as f32,
            3 => TRIPLE as f32,
            4 => QUAD as f32,
            5 => PENTA as f32,
            _ => PENTA as f32 + (lines - 5) as f32,
        },
        SpinType::Mini => match lines {
            0 => SPIN_MINI as f32,
            1 => SPIN_MINI_SINGLE as f32,
            2 => SPIN_MINI_DOUBLE as f32,
            3 => SPIN_MINI_TRIPLE as f32,
            4 => SPIN_MINI_QUAD as f32,
            5 => SPIN_PENTA as f32,
            _ => SPIN_PENTA as f32 + 2.0 * (lines - 5) as f32,
        },
        SpinType::Full => match lines {
            0 => SPIN as f32,
            1 => SPIN_SINGLE as f32,
            2 => SPIN_DOUBLE as f32,
            3 => SPIN_TRIPLE as f32,
            4 => SPIN_QUAD as f32,
            5 => SPIN_PENTA as f32,
            _ => SPIN_PENTA as f32 + 2.0 * (lines - 5) as f32,
        },
    }
}

/// logarithmic B2B chaining bonus
fn b2b_chaining_bonus(b2b: u8) -> f32 {
    let log_part = (f32::from(b2b) * B2B_CHAINING_LOG).ln_1p();
    let floored = (1.0 + log_part).floor();
    let third = if b2b == 1 {
        0.0
    } else {
        (1.0 + log_part.fract()) / 3.0
    };
    BACK_TO_BACK_BONUS as f32 * (floored + third)
}

/// apply combo bonus based on combo table mode
fn apply_combo(base: f32, combo: u8, table: ComboTable) -> f32 {
    if combo <= 1 {
        return base;
    }
    let combo_steps = combo.saturating_sub(1) as f32;

    match table {
        ComboTable::Multiplier => {
            let multiplied = base * (1.0 + COMBO_BONUS * combo_steps);
            if combo > 2 {
                let log_floor = (combo_steps * COMBO_MINIFIER_LOG).ln_1p();
                f32::max(multiplied, log_floor)
            } else {
                multiplied
            }
        }
        ComboTable::Classic => {
            let idx = (combo as usize).saturating_sub(2).min(CLASSIC_COMBO_TABLE.len() - 1);
            base + CLASSIC_COMBO_TABLE[idx] as f32
        }
        ComboTable::Modern => {
            let idx = (combo as usize).saturating_sub(2).min(MODERN_COMBO_TABLE.len() - 1);
            base + MODERN_COMBO_TABLE[idx] as f32
        }
        ComboTable::None => base,
    }
}

/// TETR.IO S2 attack calculation
/// returns garbage lines sent as f32 (caller truncates as needed)
pub fn calculate_attack(
    lines: u8,
    spin: SpinType,
    b2b: u8,
    combo: u8,
    config: AttackConfig,
    is_perfect_clear: bool,
) -> f32 {
    calculate_attack_full(AttackContext {
        lines,
        spin,
        b2b,
        combo,
        config,
        is_perfect_clear,
        b2b_broken_from: None,
        clears_garbage: false,
    })
}

#[derive(Debug)]
pub struct AttackContext {
    pub lines: u8,
    pub spin: SpinType,
    pub b2b: u8,
    pub combo: u8,
    pub config: AttackConfig,
    pub is_perfect_clear: bool,
    /// If Some(prev_b2b) and prev_b2b >= 4, a non-difficult clear just broke
    /// a long B2B chain — release stored surge as bonus attack.
    pub b2b_broken_from: Option<u8>,
    /// If true and the clear is b2b-eligible, add +1.
    pub clears_garbage: bool,
}

/// Extended attack calculation with surge release and garbage clear boost.
pub fn calculate_attack_full(ctx: AttackContext) -> f32 {
    let AttackContext {
        lines,
        spin,
        b2b,
        combo,
        config,
        is_perfect_clear,
        b2b_broken_from,
        clears_garbage,
    } = ctx;
    if lines == 0 {
        return 0.0;
    }

    let mut attack = base_attack(lines, spin);

    let is_b2b_eligible = spin != SpinType::None || lines >= 4;
    let b2b_for_bonus = if is_b2b_eligible { b2b } else { 0 };

    if b2b_for_bonus > 0 {
        if config.b2b_chaining {
            attack += b2b_chaining_bonus(b2b_for_bonus);
        } else {
            attack += BACK_TO_BACK_BONUS as f32;
        }
    }

    // combo
    let effective_combo = combo.saturating_add(1);
    attack = apply_combo(attack, effective_combo, config.combo_table);

    // garbage multiplier
    attack *= config.garbage_multiplier;

    // garbage clear boost: difficult clear that also clears garbage
    if clears_garbage && is_b2b_eligible {
        attack += 1.0;
    }

    // surge release: non-difficult clear breaks a long B2B chain
    if config.b2b_charging
        && let Some(prev_b2b) = b2b_broken_from
            && prev_b2b > B2B_CHARGE_AT
            && !is_b2b_eligible
        {
            let charge = (prev_b2b - B2B_CHARGE_AT + B2B_CHARGE_BASE) as f32;
            attack += (charge * config.garbage_multiplier).floor();
        }

    // perfect clear bonus (added after multiplier, like damcalc.js)
    if is_perfect_clear {
        attack += config.pc_garbage as f32;
    }

    attack
}
