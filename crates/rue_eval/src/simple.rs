//! Simple weights as a baseline comparison.
use rue_core::{game::Game, placement::Move};

use crate::{
    features::{self},
    weights::Weights,
};

/// Simple weights as a baseline comparison.
pub struct Simple {
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
}

impl Weights for Simple {
    fn evaluate<const N: usize>(&self, game: &Game<N>, _placement: &Move) -> f64 {
        let mut score = 0.0;
        let heights = features::heights(&game.board);
        let max_height = *heights.iter().max().unwrap() as f64;

        let (well_col, _) = features::find_well(&heights);
        let (bumpiness, bumpiness_sq) = features::bumpiness(&heights, well_col);
        let (holes, covered) = features::holes_and_covered(&game.board, &heights);

        score += self.height * max_height;

        if max_height > 10.0 {
            score += self.height_half * (max_height - 10.0);
        }

        if max_height > 15.0 {
            score += self.height_three_quarters * (max_height - 15.0);
        }
        
        score += self.bumpiness * bumpiness as f64;
        score += self.bumpiness_sq * bumpiness_sq as f64;
        score += self.cell_coveredness * covered as f64;
        score += self.holes * holes as f64;

        score
    }

    fn flatten(&self) -> Vec<f64> {
        vec![self.height]
    }
}
