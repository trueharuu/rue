use rue_core::game::Game;
use rue_core::placement::Move;
use rustc_hash::FxHashMap;

/// Configuration for beam search.
pub struct SearchConfig {
    /// Number of nodes kept at each beam level.
    pub beam_width: usize,
    /// Maximum search depth (number of placements ahead).
    pub depth: usize,
    /// Score delta for futility pruning. Nodes with score < best - delta are dropped.
    /// Set to `0.0` to disable.
    pub futility_delta: f64,
    /// Optional time budget in milliseconds. When set, enables iterative widening:
    /// starts with a narrow beam and doubles until the budget expires or max width
    /// is reached.
    pub time_budget_ms: Option<u64>,
    /// Multiplier for the offensive attack term.
    pub attack_weight: f64,
    /// Multiplier for the chain/B2B maintenance term.
    pub chain_weight: f64,
    /// Multiplier for the core board evaluation term.
    pub board_weight: f64,
    /// Cap for sqrt(depth) normalization of cumulative attack/chain terms.
    pub max_depth_factor: f64,
    /// Maximum additional depths to extend "loud" nodes (mid-combo, active B2B).
    pub quiescence_max_extensions: usize,
    /// Fraction of `beam_width` allocated to quiescence extension beam.
    pub quiescence_beam_fraction: f64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            beam_width: 800,
            depth: 14,
            futility_delta: 15.0,
            time_budget_ms: None,
            attack_weight: 0.50,
            chain_weight: 0.15,
            board_weight: 1.0,
            max_depth_factor: 2.45,
            quiescence_max_extensions: 3,
            quiescence_beam_fraction: 0.15,
        }
    }
}

/// A node in the search tree.
#[derive(Clone)]
pub struct SearchNode<const N: usize> {
    /// The game state at this node.
    pub game: Game<N>,
    /// Composite evaluation score.
    pub score: f64,
    /// The root-level move that originated this path.
    pub root_move: Move,
    /// Whether the root move used the hold slot.
    pub root_hold_used: bool,
    /// Sequence of moves from root to this node.
    pub path: Vec<Move>,
    /// Single-move attack value for this node's placement.
    pub attack_score: f64,
    /// Chain/B2B maintenance bonus for this node's placement.
    pub chain_score: f64,
    /// Static board evaluation score.
    pub board_score: f64,
    /// Cumulative attack value along the search path.
    pub path_attack: f64,
    /// Cumulative chain value along the search path.
    pub path_chain: f64,
}

impl<const N: usize> SearchNode<N> {
    /// A node is "loud" if it has unresolved tactical activity that makes
    /// leaf evaluation unreliable — analogous to chess quiescence search
    /// refusing to evaluate mid-capture positions.
    #[must_use]
    #[inline]
    pub fn is_loud(&self) -> bool {
        self.game.combo_count.is_some() || self.game.b2b_count.is_some_and(|b| b > 0)
    }
}

/// Result of a beam search.
pub struct SearchResult<const N: usize> {
    /// The best leaf node found.
    pub best: SearchNode<N>,
}

/// Extended search result with per-root-move scores from the final beam.
pub struct SearchResultFull<const N: usize> {
    /// The best result.
    pub best: SearchResult<N>,
    /// (`root_move`, `best_leaf_score`) for every root move that survived to the
    /// final beam. Sorted descending by score.
    pub root_scores: Vec<(Move, f64)>,
    /// Position complexity: variance of top-10 `root_scores`.
    pub position_complexity: f64,
    /// Static board evaluation score of the best node.
    pub board_score: f64,
    /// Attack value of the best node's placement.
    pub attack_score: f64,
    /// Chain score of the best node's placement.
    pub chain_score: f64,
    /// Cumulative attack along the best path.
    pub path_attack: f64,
    /// Cumulative chain along the best path.
    pub path_chain: f64,
}

/// Shared context for node expansion functions.
pub(crate) struct SearchExpansionContext<'a, W, const N: usize> {
    pub config: &'a SearchConfig,
    pub weights: &'a W,
    pub remaining_depth: usize,
    pub tt: &'a mut Option<FxHashMap<u64, (u8, f64)>>,
}

/// Parameters for a single beam search iteration.
pub(crate) struct SearchIterationParams<'a, W, const N: usize> {
    pub game: &'a Game<N>,
    pub config: &'a SearchConfig,
    pub weights: &'a W,
    pub max_depth: usize,
    pub beam_width: usize,
    pub tt: &'a mut Option<FxHashMap<u64, (u8, f64)>>,
    pub forced_root_move: Option<Move>,
}
