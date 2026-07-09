/// Configuration for beam search.
pub struct SearchConfig {
    /// Number of nodes kept at each beam level.
    pub beam_width: usize,
    /// Maximum search depth (number of placements ahead).
    pub depth: usize,
    /// Score delta for futility pruning. Nodes with score < best - delta are dropped.
    /// Set to `0.0` to disable.
    pub futility_delta: f64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            beam_width: 100,
            depth: 4,
            futility_delta: 0.0,
        }
    }
}
