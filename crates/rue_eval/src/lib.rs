//! Evaluation engine for singular board positions.
//! This allows us to determine a relative "goodness" of a board, to be accentuated by `rue_search`.
pub mod simple;
pub mod weights;
pub mod features;

/// Normalize a value to the range [-1.0, 1.0].
pub fn normalize(x: f64) -> f64 {
    2.0 * (x - (-1.0)) / (1.0 - (-1.0)) - 1.0
}
