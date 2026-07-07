//! `Weights` trait.

use rue_core::{game::Game, placement::Move};

/// A series of weights.
pub trait Weights {
    /// Evaluate a singular position on the board and return a score.
    fn evaluate<const N: usize>(&self, game: &Game<N>, placement: &Move) -> f64;

    /// Return the weights as a vector of f64 values.
    fn flatten(&self) -> Vec<f64>;
}