use engine_core::game::Game;

use crate::features::{bumpiness, column_heights, count_tsd_overhangs, find_well, holes_and_covered, row_transitions};

#[derive(Clone, Copy)]
pub struct BoardModel {
    pub height: f64,
    pub height_half: f64,
    pub height_quar: f64,

    pub holes: f64,
    pub cell_coveredness: f64,
    pub bumpiness: f64,
    pub bumpiness_sq: f64,
    pub row_transitions: f64,

    pub well_column: [f64; 10],
    pub well_depth: f64,

    pub tsd_overhangs: f64,
}

impl BoardModel {
    #[must_use]
    pub fn eval(&self, game: &Game) -> f64 {
        let mut score = 0.0;
        let heights = column_heights(&game.board);
        let max_height = heights.iter().copied().max().unwrap_or(0) as f64;

        let (holes, covered) = holes_and_covered(&game.board, &heights);
        let (well_col, well_depth) = find_well(&heights);
        let (bump, bump_sq) = bumpiness(&heights, well_col);
        let rt = row_transitions(&game.board, max_height as usize);

        score += self.height * max_height;
        if max_height > 10.0 {
            score += self.height_half * (max_height - 10.0);
        }
        if max_height > 15.0 {
            score += self.height_quar * (max_height - 15.0);
        }

        score += self.holes * f64::from(holes);
        score += self.cell_coveredness * f64::from(covered);
        score += self.bumpiness * f64::from(bump);
        score += self.bumpiness_sq * f64::from(bump_sq);
        score += self.row_transitions * f64::from(rt);


        if let Some(well_x) = well_col {
            score += self.well_column[well_x] * 1.0;
            score += self.well_depth * f64::from(well_depth);
        }

        let tsd_overhangs = count_tsd_overhangs(&game.board, &heights);
        score += self.tsd_overhangs * f64::from(tsd_overhangs);

        

        score
    }
}
