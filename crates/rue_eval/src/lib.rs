//! Evaluation engine for singular board positions.
//! This allows us to determine a relative "goodness" of a board, to be accentuated by `rue_search`.
pub mod simple;
pub mod weights;
pub mod features;

