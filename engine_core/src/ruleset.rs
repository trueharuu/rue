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
    enable_allspin: true,
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
pub const SPIN_QUAD: u8 = 10;
pub const SPIN_PENTA: u8 = 12;

pub const BACK_TO_BACK_BONUS: u8 = 1;
const B2B_CHAINING_LOG: f32 = 0.8;
const COMBO_BONUS: f32 = 0.25;
const COMBO_FLOOR_SCALE: f32 = 1.25;

const CLASSIC_COMBO_TABLE: [u8; 11] = [0, 1, 1, 2, 2, 3, 3, 4, 4, 4, 5];
const MODERN_COMBO_TABLE: [u8; 13] = [0, 1, 1, 2, 2, 2, 3, 3, 3, 3, 3, 3, 4];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComboTable {
    Multiplier,
    Classic,
    Modern,
    None,
}

#[derive(Debug, Clone)]
pub struct AttackConfig {
    pub pc_garbage: u8,
    pub pc_b2b: u8,
    pub b2b_chaining: bool,
    pub combo_table: ComboTable,
    pub garbage_multiplier: f32,
}

impl AttackConfig {
    pub fn tetra_league() -> Self {
        Self {
            pc_garbage: 5,
            pc_b2b: 2,
            b2b_chaining: true,
            combo_table: ComboTable::Multiplier,
            garbage_multiplier: 1.0,
        }
    }

    pub fn quick_play() -> Self {
        Self {
            pc_garbage: 3,
            pc_b2b: 2,
            b2b_chaining: false,
            combo_table: ComboTable::Multiplier,
            garbage_multiplier: 1.0,
        }
    }
}

/// base garbage for a line clear + spin type (piece-agnostic)
fn base_attack(lines: u8, spin: SpinType) -> f32 {
    match spin {
        SpinType::NoSpin => match lines {
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
            4 => SPIN_QUAD as f32,
            _ => SPIN_QUAD as f32 + 2.0 * (lines - 4) as f32,
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

/// logarithmic B2B chaining bonus (S2 surge mechanic)
fn b2b_chaining_bonus(b2b: u8) -> f32 {
    if b2b <= 1 {
        return BACK_TO_BACK_BONUS as f32;
    }
    // floor(1 + ln(1 + b2b * B2B_CHAINING_LOG)) with fractional third
    let log_part = (1.0 + b2b as f32 * B2B_CHAINING_LOG).ln();
    let floored = (1.0 + log_part).floor();
    // fractional third: the remainder after floor contributes a third
    let remainder = (1.0 + log_part) - floored;
    let third = if remainder > 0.0 {
        remainder / 3.0
    } else {
        0.0
    };
    floored + third
}

/// apply combo bonus based on combo table mode
fn apply_combo(base: f32, combo: u8, table: ComboTable) -> f32 {
    if combo == 0 {
        return base;
    }

    match table {
        ComboTable::Multiplier => {
            let multiplied = base * (1.0 + COMBO_BONUS * combo as f32);
            // for combo > 1, log floor is a MINIMUM guarantee (matches Triangle.js)
            if combo > 1 {
                let log_floor = (1.0 + combo as f32 * COMBO_FLOOR_SCALE).ln();
                f32::max(multiplied, log_floor)
            } else {
                multiplied
            }
        }
        ComboTable::Classic => {
            let idx = (combo as usize).min(CLASSIC_COMBO_TABLE.len() - 1);
            base + CLASSIC_COMBO_TABLE[idx] as f32
        }
        ComboTable::Modern => {
            let idx = (combo as usize).min(MODERN_COMBO_TABLE.len() - 1);
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
    config: &AttackConfig,
    is_perfect_clear: bool,
) -> f32 {
    calculate_attack_full(&AttackContext {
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

/// Parameters for the extended attack calculation.
pub struct AttackContext<'a> {
    pub lines: u8,
    pub spin: SpinType,
    pub b2b: u8,
    pub combo: u8,
    pub config: &'a AttackConfig,
    pub is_perfect_clear: bool,
    /// If Some(prev_b2b) and prev_b2b >= 4, a non-difficult clear just broke
    /// a long B2B chain — release stored surge as bonus attack.
    pub b2b_broken_from: Option<u8>,
    /// If true and the clear is b2b-eligible, add +1.
    pub clears_garbage: bool,
}

/// Extended attack calculation with surge release and garbage clear boost.
pub fn calculate_attack_full(ctx: &AttackContext<'_>) -> f32 {
    let AttackContext {
        lines,
        spin,
        b2b,
        combo,
        config,
        is_perfect_clear,
        b2b_broken_from,
        clears_garbage,
    } = *ctx;
    if lines == 0 {
        return 0.0;
    }

    let mut attack = base_attack(lines, spin);

    // perfect clear bonus
    if is_perfect_clear {
        attack += config.pc_garbage as f32;
    }

    let is_b2b_eligible = spin != SpinType::NoSpin || lines >= 4;

    // B2B bonus: trust the caller's b2b value — eligibility is enforced
    // upstream (engine resets b2b to -1 for non-eligible clears). A positive
    // b2b here is always legitimate (e.g. PC preserves the chain).
    if b2b > 0 {
        if config.b2b_chaining {
            attack += b2b_chaining_bonus(b2b);
        } else {
            attack += BACK_TO_BACK_BONUS as f32;
        }
    }

    // perfect clear B2B bonus (separate from regular B2B)
    if is_perfect_clear && b2b > 0 {
        attack += config.pc_b2b as f32;
    }

    // surge release: non-difficult clear breaks a long B2B chain
    if let Some(prev_b2b) = b2b_broken_from
        && prev_b2b >= 4 && !is_b2b_eligible {
            attack += 4.0 + (prev_b2b - 4) as f32;
        }

    // garbage clear boost: difficult clear that also clears garbage
    if clears_garbage && is_b2b_eligible {
        attack += 1.0;
    }

    // combo
    attack = apply_combo(attack, combo, config.combo_table);

    // garbage multiplier
    attack *= config.garbage_multiplier;

    attack
}