//! Simple weights as a baseline comparison.
use rue_core::game::{Game, attack::AttackContext};

use crate::{
    features::{self},
    tunable::Tunable,
    weights::Weights,
};

/// Simple weights as a baseline comparison.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Simple {
    /// Current B2B of the game.
    pub b2b: f64,
    /// Current combo of the game.
    pub combo: f64,
    /// Total incoming garbage in the queue.
    pub garbage: f64,
    /// The height of the board.
    pub height: f64,
    /// The height of the board, from the midpoint to the top.
    pub height_half: f64,
    /// The height of the board, from the third quartile to the top.
    pub height_three_quarters: f64,
    /// The sum of height changes outside a well.
    pub bumpiness: f64,
    /// The sum of squares of height changes outside a well.
    pub bumpiness_sq: f64,
    /// The number of cells covered by blocks.
    pub cell_coveredness: f64,
    /// The number of holes in the board.
    pub holes: f64,
    /// Count of row transitions.
    pub row_transitions: f64,
    /// Weights for each permutation of (line clears, spin)
    pub active: [[f64; 3]; 5],
    /// Total attack sent for this clear.
    pub sent: f64,
    /// Where the well is located, if any
    pub well_col: [f64; 10],
    /// Total depth of the highest well
    pub well_depth: f64,
}

impl Weights for Simple {
    fn name() -> &'static str {
        "simple"
    }

    fn evaluate<const N: usize>(&self, game: &Game<N>, ctx: &AttackContext) -> f64 {
        let mut score = 0.0;
        let heights = features::heights(&game.board);
        let max_height = *heights.iter().max().unwrap() as f64;

        let (well_col, well_depth) = features::find_well(&heights);

        if let Some(s) = well_col {
            score += self.well_col[s];
            score += self.well_depth * f64::from(well_depth);
        }

        let (bumpiness, bumpiness_sq) = features::bumpiness(&heights, well_col);
        let (holes, covered) = features::holes_and_covered(&game.board, &heights);

        score += self.height * max_height;

        if max_height > 10.0 {
            score += self.height_half * (max_height - 10.0);
        }

        if max_height > 15.0 {
            score += self.height_three_quarters * (max_height - 15.0);
        }

        score += self.bumpiness * f64::from(bumpiness);
        score += self.bumpiness_sq * f64::from(bumpiness_sq);
        score += self.cell_coveredness * f64::from(covered);
        score += self.holes * f64::from(holes);

        score += self.row_transitions
            * f64::from(features::row_transitions(&game.board, max_height as usize));
        if let Some(b) = game.b2b_count {
            score += self.b2b * f64::from(b.clamp(0, 8));
        }

        score += self.active[ctx.lines_cleared as usize][ctx.placement.spin() as usize];
        score += self.sent * f64::from(ctx.attack_sent);

        score += self.combo * f64::from(game.combo_count.unwrap_or(0));
        score += self.garbage * f64::from(game.garbage_queue.total());

        score
    }
}

/// Parameter names for `Simple`, indexed 0..38.
const SIMPLE_PARAM_NAMES: [&str; 38] = [
    "b2b",
    "combo",
    "garbage",
    "height",
    "height_half",
    "height_three_quarters",
    "bumpiness",
    "bumpiness_sq",
    "cell_coveredness",
    "holes",
    "row_transitions",
    "active[0][0]",
    "active[0][1]",
    "active[0][2]",
    "active[1][0]",
    "active[1][1]",
    "active[1][2]",
    "active[2][0]",
    "active[2][1]",
    "active[2][2]",
    "active[3][0]",
    "active[3][1]",
    "active[3][2]",
    "active[4][0]",
    "active[4][1]",
    "active[4][2]",
    "sent",
    "well_col[0]",
    "well_col[1]",
    "well_col[2]",
    "well_col[3]",
    "well_col[4]",
    "well_col[5]",
    "well_col[6]",
    "well_col[7]",
    "well_col[8]",
    "well_col[9]",
    "well_depth",
];

/// Per-parameter `(min, max)` bounds for `Simple`, indexed 0..38.
const SIMPLE_PARAM_BOUNDS: [(f64, f64); 38] = [(-1.0, 1.0); 38];

impl Tunable for Simple {
    fn param_count() -> usize {
        38
    }

    fn get_param(&self, i: usize) -> f64 {
        match i {
            0 => self.b2b,
            1 => self.combo,
            2 => self.garbage,
            3 => self.height,
            4 => self.height_half,
            5 => self.height_three_quarters,
            6 => self.bumpiness,
            7 => self.bumpiness_sq,
            8 => self.cell_coveredness,
            9 => self.holes,
            10 => self.row_transitions,
            11..=25 => {
                let idx = i - 11;
                self.active[idx / 3][idx % 3]
            }
            26 => self.sent,
            27..=36 => self.well_col[i - 27],
            37 => self.well_depth,
            _ => panic!("Simple::get_param: index {i} out of range [0, 38)"),
        }
    }

    fn set_param(&mut self, i: usize, v: f64) {
        match i {
            0 => self.b2b = v,
            1 => self.combo = v,
            2 => self.garbage = v,
            3 => self.height = v,
            4 => self.height_half = v,
            5 => self.height_three_quarters = v,
            6 => self.bumpiness = v,
            7 => self.bumpiness_sq = v,
            8 => self.cell_coveredness = v,
            9 => self.holes = v,
            10 => self.row_transitions = v,
            11..=25 => {
                let idx = i - 11;
                self.active[idx / 3][idx % 3] = v;
            }
            26 => self.sent = v,
            27..=36 => self.well_col[i - 27] = v,
            37 => self.well_depth = v,
            _ => panic!("Simple::set_param: index {i} out of range [0, 38)"),
        }
    }

    fn param_name(i: usize) -> &'static str {
        SIMPLE_PARAM_NAMES[i]
    }

    fn param_bounds(i: usize) -> (f64, f64) {
        SIMPLE_PARAM_BOUNDS[i]
    }

    fn to_vec(&self) -> Vec<f64> {
        let mut v = Vec::with_capacity(38);
        for i in 0..38 {
            v.push(self.get_param(i));
        }
        v
    }

    fn from_vec(v: &[f64]) -> Self {
        assert_eq!(
            v.len(),
            38,
            "Simple::from_vec: expected 38 parameters, got {}",
            v.len()
        );
        let mut s = Self {
            b2b: v[0],
            combo: v[1],
            garbage: v[2],
            height: v[3],
            height_half: v[4],
            height_three_quarters: v[5],
            bumpiness: v[6],
            bumpiness_sq: v[7],
            cell_coveredness: v[8],
            holes: v[9],
            row_transitions: v[10],
            active: [[0.0; 3]; 5],
            sent: v[26],
            well_col: [0.0; 10],
            well_depth: v[37],
        };

        for idx in 0..15 {
            s.active[idx / 3][idx % 3] = v[11 + idx];
        }
        s.well_col.copy_from_slice(&v[27..37]);
        s
    }
}
