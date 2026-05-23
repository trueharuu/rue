use engine_core::placement::Move;
use engine_eval::Model;

pub struct SearchConfig {
    pub width: usize,
    pub depth: usize,

    pub model: Model,
}

pub struct SearchResult {
    pub best_move: Move,
    pub score: f64,
}
