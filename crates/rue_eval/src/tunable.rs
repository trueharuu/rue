//! `Tunable` trait for parameter-level access to weight models.

use crate::weights::Weights;

/// A [`Weights`] implementation that exposes its parameters for optimization.
///
/// SPSA and similar optimizers operate on a flat `Vec<f64>` of parameters.
/// This trait bridges between the typed struct and the flat representation.
pub trait Tunable: Weights + Clone {
    /// Total number of tunable scalar parameters.
    fn param_count() -> usize;

    /// Read parameter at index `i` (0-based, row-major for nested arrays).
    fn get_param(&self, i: usize) -> f64;

    /// Write parameter at index `i`.
    fn set_param(&mut self, i: usize, v: f64);

    /// Human-readable name for parameter `i` (for logging / CSV output).
    fn param_name(i: usize) -> &'static str;

    /// Per-parameter bounds as `(min, max)`. Index must match `get_param`/`set_param` order.
    fn param_bounds(i: usize) -> (f64, f64);

    /// Bulk export all parameters as a `Vec<f64>` (length == `param_count()`).
    fn to_vec(&self) -> Vec<f64> {
        (0..Self::param_count())
            .map(|i| self.get_param(i))
            .collect()
    }

    /// Bulk import from a `Vec<f64>`. Panics if `len != param_count()`.
    fn from_vec(v: &[f64]) -> Self;
}
