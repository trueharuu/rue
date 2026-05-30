use rue_core::placement::Move;
use rue_eval::Model;

pub struct SearchConfig {
    pub width: usize,
    pub depth: usize,

    pub model: Model,

    pub futility_delta: f64,
    pub time_budget_ms: Option<u64>,
    pub use_tt: bool,
    pub extend_queue_7bag: bool,
    pub quiescence_max_extensions: usize,
    pub quiescence_beam_fraction: f64,
}

impl SearchConfig {
    pub const DEFAULT_FUTILITY_DELTA: f64 = 15.0;
    pub const DEFAULT_TIME_BUDGET_MS: u64 = 25;
    pub const DEFAULT_QUIESCENCE_MAX_EXTENSIONS: usize = 3;
    pub const DEFAULT_QUIESCENCE_BEAM_FRACTION: f64 = 0.15;

    #[must_use]
    pub fn new(model: Model, width: usize, depth: usize, target_ms: Option<u64>) -> Self {
        Self {
            width,
            depth,
            model,
            futility_delta: Self::DEFAULT_FUTILITY_DELTA,
            time_budget_ms: target_ms,
            use_tt: false,
            extend_queue_7bag: true,
            quiescence_max_extensions: Self::DEFAULT_QUIESCENCE_MAX_EXTENSIONS,
            quiescence_beam_fraction: Self::DEFAULT_QUIESCENCE_BEAM_FRACTION,
        }
    }
}

pub struct SearchResult {
    pub best_move: Move,
    pub score: f64,
}
