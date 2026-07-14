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

/// Hyperparameters for the SPSA algorithm.
#[allow(non_snake_case)]
pub struct SpsaConfig {
    /// Step-size numerator. Controls how large updates are.
    pub a0: f64,
    /// Perturbation-size numerator. Controls estimation noise.
    pub c0: f64,
    /// Stability constant (A). Should be ~10% of expected max iterations.
    /// Larger values make early gain sequences decay more slowly.
    pub A: f64,
    /// Exponent for `a_k` gain sequence. Standard SPSA: 0.602.
    pub alpha: f64,
    /// Exponent for `c_k` gain sequence. Standard SPSA: 0.101.
    pub gamma: f64,
    /// Maximum number of SPSA iterations.
    pub max_iter: usize,
    /// Fitness evaluation parameters (games, pieces, beam config).
    pub fitness: FitnessConfig,
}

impl Default for SpsaConfig {
    fn default() -> Self {
        Self {
            a0: 0.05,
            c0: 0.1,
            A: 10.0,
            alpha: 0.602,
            gamma: 0.101,
            max_iter: 200,
            fitness: FitnessConfig::default(),
        }
    }
}
