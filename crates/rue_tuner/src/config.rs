//! Configuration for Rue's fitness evaluation.
use rue_search::SearchConfig;

/// Configuration for fitness evaluation.
pub struct FitnessConfig {
    /// Number of pieces to play per game.
    pub pieces: usize,
    /// Number of games to average over.
    pub games: usize,
    /// Beam width for search.
    pub beam_width: usize,
    /// Search depth.
    pub depth: usize,
}

impl Default for FitnessConfig {
    fn default() -> Self {
        Self {
            pieces: 500,
            games: 8,
            beam_width: 500,
            depth: 7,
        }
    }
}

impl FitnessConfig {
    /// Build a [`SearchConfig`] from these fitness parameters.
    #[must_use]
    pub fn search_config(&self) -> SearchConfig {
        SearchConfig {
            beam_width: self.beam_width,
            depth: self.depth,
            futility_delta: 0.0,
            time_budget_ms: None,
            ..SearchConfig::default()
        }
    }
}
