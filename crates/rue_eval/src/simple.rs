//! Simple weights as a baseline comparison.
use rue_core::game::Game;
use rue_core::game::attack::AttackContext;
use rue_core::piece::Piece;
use rue_core::spin::Spin;

use crate::features::{self};
use crate::weights::Weights;

/// Simple weights as a baseline comparison.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Simple {
    /// Current B2B of the game.
    pub b2b: f64,
    /// Current combo of the game.
    pub combo: f64,
    /// Whether the resulting board is a Perfect Clear.
    pub pc: f64,
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
    /// Total number of T-Spin overhangs
    pub tsd_overhangs: f64,
    /// Whether a piece is wasted (non-spin/quad clear)
    pub waste: [f64; Piece::NB],
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
            score += self.b2b * f64::from(b.clamp(0, 20));
        }

        score += self.active[ctx.lines_cleared as usize][ctx.placement.spin() as usize];
        score += self.sent * f64::from(ctx.attack_sent);

        score += self.combo * f64::from(game.combo_count.unwrap_or(0));
        score += self.garbage * f64::from(game.garbage_queue.total());

        score += self.pc * f64::from(!game.board.any());
        score += self.tsd_overhangs * f64::from(features::tsd_overhangs(&game.board, &heights));

        // a piece is not waste if
        // its a spin clear with >0 lines cleared
        // or if exactly 4 lines cleared (quad clear)
        let is_waste = !((ctx.placement.spin() != Spin::None && ctx.lines_cleared > 0)
            || ctx.lines_cleared == 4);

        if is_waste {
            score += self.waste[ctx.placement.piece() as usize];
        }
        score
    }
}
