//! Search configuration and results.

use std::time::Duration;

use rue_core::placement::Move;

/// Parameters for a single beam search decision.
#[derive(Clone, Copy, Debug)]
pub struct SearchConfig {
    /// Final beam width. With a time budget the search widens up to this value.
    pub beam_width: usize,
    /// Number of pieces placed per line.
    pub depth: usize,
    /// Time budget for iterative widening. `None` searches once at full width.
    pub time_budget: Option<Duration>,
    /// Drop children below `level_max - futility_delta`. `0` disables it.
    pub futility_delta: f32,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            beam_width: 800,
            depth: 7,
            time_budget: None,
            futility_delta: 15.0,
        }
    }
}

/// The chosen root placement and its score.
#[derive(Clone, Copy, Debug)]
pub struct SearchResult {
    /// The first placement that leads to the best found line.
    pub best_move: Move,
    /// Score of the final board on that line.
    pub score: f32,
}