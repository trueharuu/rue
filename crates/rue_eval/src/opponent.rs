use rue_core::game::Game;
use serde::{Deserialize, Serialize};

use crate::features::{bumpiness, column_heights, find_well, holes_and_covered};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct OpponentModel {
    pub well_depth: f64,
    pub height: f64,
    pub bumpiness: f64,
    pub holes: f64,
}

impl OpponentModel {
    #[must_use]
    pub fn eval(&self, game: &Game) -> f64 {
        let mut score = 0.0;

        let heights = column_heights(&game.board);
        let (well_col, well_depth) = find_well(&heights);

        if well_col.is_some() {
            score += self.well_depth * f64::from(well_depth);
        }

        score += self.height * f64::from(heights.iter().copied().max().unwrap_or(0) as u8);

        let (bumpiness, _) = bumpiness(&heights, well_col);

        score += self.bumpiness * f64::from(bumpiness);
        
        let (holes, _) = holes_and_covered(&game.board, &heights);
        score += self.holes * f64::from(holes);

        score
    }
}
