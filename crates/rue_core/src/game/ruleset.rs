//! Module containing the [`Ruleset`] structure and logic.
use crate::spin::Spin;
use crate::spin::Spins;

/// A ruleset for any given game.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ruleset {
    /// The spins allowed in this ruleset.
    pub spins: Spins,
    /// Attack sent for a single.
    pub single: u32,
    /// Attack sent for a double.
    pub double: u32,
    /// Attack sent for a triple.
    pub triple: u32,
    /// Attack sent for a quad.
    pub quad: u32,
    /// Attack sent for a penta. This should not typically be used.
    pub penta: u32,
    /// Attack sent for a spin-mini-zero.
    pub spin_mini_zero: u32,
    /// Attack sent for a spin-zero.
    pub spin_zero: u32,
    /// Attack sent for a spin-mini-single.
    pub spin_mini_single: u32,
    /// Attack sent for a spin-single.
    pub spin_single: u32,
    /// Attack sent for a spin-mini-double.
    pub spin_mini_double: u32,
    /// Attack sent for a spin-double.
    pub spin_double: u32,
    /// Attack sent for a spin-mini-triple.
    pub spin_mini_triple: u32,
    /// Attack sent for a spin-triple.
    pub spin_triple: u32,
    /// Attack sent for a spin-quad. This should not typically be used.
    pub spin_quad: u32,
    /// Attack sent for a spin-penta. This should not typically be used.
    pub spin_penta: u32,
    /// Bonus attack sent for having back-to-back while surge is active.
    pub back_to_back_bonus: u32,
    /// The logarithmic scaling factor for back-to-back chaining.
    pub b2b_chaining_log: f64,
    /// Bonus attack multiplier dependent on combo.
    pub combo_bonus: f64,
    /// The scaling factor for combo chaining.
    pub combo_floor_scale: f64,
    /// Garbage sent when a perfect clear is achieveed.
    pub pc_garbage: u32,
    /// Back-to-back gained when a perfect clear is achieved. If [`None`], non-B2B perfect clears will break B2B.
    pub pc_b2b: Option<u32>,
    /// Whether to apply a chaining bonus for long back-to-back chains.
    pub b2b_chaining: bool,
    /// Whether to apply a surge bonus for long back-to-back chains, which sends the current B2B on the next non-B2B line clear.
    pub b2b_charging: bool,
    /// Global garbage multiplier.
    pub garbage_multiplier: f64,
    /// Smallest B2B value that starts surge.
    pub b2b_charging_start: u32,
    /// Bonus for clearing garbage with a quad or a spin clear.
    pub garbage_clear_bonus: u32,
    /// Max amount of garbage that can be tanked per placement.
    pub garbage_cap: u32,
    /// Max amount of garbage, total
    pub garbage_absolute_cap: u32,
    /// Whether to use SRS+ (TETR.IO) kick tables instead of standard SRS.
    pub srs_plus: bool,
    /// Whether to enable 180-degree rotation.
    pub use_180: bool,
    /// Whether to use infinite SDF for move generation.
    pub inf_sdf: bool,
}

impl Ruleset {
    /// Returns base garbage before combo, B2B, surge, and multipliers.
    #[must_use]
    pub fn base_attack(&self, lines: u32, spin: Spin) -> u32 {
        match spin {
            Spin::None => match lines {
                0 => 0,
                1 => self.single,
                2 => self.double,
                3 => self.triple,
                4 => self.quad,
                5 => self.penta,
                _ => self.penta + (lines - 5),
            },
            Spin::Mini => match lines {
                0 => self.spin_mini_zero,
                1 => self.spin_mini_single,
                2 => self.spin_mini_double,
                3 => self.spin_mini_triple,
                4 => self.spin_quad,
                _ => self.spin_quad + 2 * (lines - 4),
            },
            Spin::Full => match lines {
                0 => self.spin_zero,
                1 => self.spin_single,
                2 => self.spin_double,
                3 => self.spin_triple,
                4 => self.spin_quad,
                5 => self.spin_penta,
                _ => self.spin_penta + 2 * (lines - 5),
            },
        }
    }
}

/// Sequential combo table for [`ComboTable::Classic`].
pub const CLASSIC_COMBO_TABLE: [u8; 11] = [0, 1, 1, 2, 2, 3, 3, 4, 4, 4, 5];
/// Sequential combo table for [`ComboTable::Modern`].
pub const MODERN_COMBO_TABLE: [u8; 13] = [0, 1, 1, 2, 2, 2, 3, 3, 3, 3, 3, 3, 4];

/// A rule sets combo table.
pub enum ComboTable {
    /// No combo bonus.
    None,
    /// Classic combo table, from older guideline games.
    Classic,
    /// Modern combo table, from newer guideline games.
    Modern,
    /// Multiplier combo table, from TETR.IO.
    Multiplier,
}

/// The currently active rule set for TETR.IO Tetra League Season 2.
pub const SEASON_2: Ruleset = Ruleset {
    spins: Spins::AllMini,
    single: 0,
    double: 1,
    triple: 2,
    quad: 4,
    penta: 5,
    spin_mini_zero: 0,
    spin_zero: 0,
    spin_mini_single: 0,
    spin_single: 2,
    spin_mini_double: 1,
    spin_double: 4,
    spin_mini_triple: 2,
    spin_triple: 6,
    spin_quad: 10,
    spin_penta: 12,
    back_to_back_bonus: 1,
    b2b_chaining_log: 0.8,
    combo_bonus: 0.5,
    combo_floor_scale: 1.25,
    b2b_chaining: false,
    b2b_charging: true,
    b2b_charging_start: 4,
    pc_b2b: Some(1),
    pc_garbage: 5,
    garbage_multiplier: 1.0,
    garbage_clear_bonus: 0,
    garbage_cap: 8,
    garbage_absolute_cap: u32::MAX,
    srs_plus: true,
    use_180: true,
    inf_sdf: true,
};
