use engine_core::{
    board::COL_NB, game::Game, piece::ALL_PIECES, placement::Move, ruleset::AttackContext,
};

use crate::{active::ActiveModel, board::BoardModel, diff::Difference};

pub mod active;
pub mod board;
pub mod diff;
pub mod features;
pub mod opponent;

use std::fmt::Write as _;
#[derive(Clone, Copy, Debug)]
pub struct Model {
    pub board: BoardModel,
    pub active: ActiveModel,
    pub board_weight: f64,
    pub active_weight: f64,
}

impl Model {
    #[must_use]
    pub fn eval(&self, game: &Game, active: &Move, ctx: &AttackContext) -> f64 {
        self.board_weight * self.board.eval(game)
            + self.active_weight * self.active.eval(active, ctx)
    }

    #[must_use]
    pub fn diff_to(&self, rhs: &Self) -> String {
        let mut f = String::new();
        writeln!(
            f,
            "height:half:quar {} {} {}",
            Difference(self.board.height, rhs.board.height),
            Difference(self.board.height_half, rhs.board.height_half),
            Difference(self.board.height_quar, rhs.board.height_quar),
        )
        .unwrap();

        writeln!(
            f,
            "holes:coveredness {} {}",
            Difference(self.board.holes, rhs.board.holes),
            Difference(self.board.cell_coveredness, rhs.board.cell_coveredness),
        )
        .unwrap();

        writeln!(
            f,
            "bumpiness:bumpiness_sq {} {}",
            Difference(self.board.bumpiness, rhs.board.bumpiness),
            Difference(self.board.bumpiness_sq, rhs.board.bumpiness_sq),
        )
        .unwrap();

        writeln!(
            f,
            "row_transitions {}",
            Difference(self.board.row_transitions, rhs.board.row_transitions),
        )
        .unwrap();

        write!(f, "well_column ").unwrap();

        for i in 0..COL_NB {
            write!(
                f,
                "{} ",
                Difference(self.board.well_column[i], rhs.board.well_column[i])
            )
            .unwrap();
        }

        writeln!(f).unwrap();
        writeln!(
            f,
            "well depth {}",
            Difference(self.board.well_depth, rhs.board.well_depth)
        )
        .unwrap();

        writeln!(
            f,
            "tsd_overhangs {}",
            Difference(self.board.tsd_overhangs, rhs.board.tsd_overhangs)
        )
        .unwrap();

        write!(f, "waste ").unwrap();

        for i in ALL_PIECES {
            write!(
                f,
                "{i:?}={} ",
                Difference(self.active.waste[i as usize], rhs.active.waste[i as usize])
            )
            .unwrap();
        }

        writeln!(f).unwrap();

        write!(f, "clear ").unwrap();

        for i in 0..5 {
            write!(
                f,
                "{} ",
                Difference(self.active.clear[i as usize], rhs.active.clear[i as usize])
            )
            .unwrap();
        }

        writeln!(f).unwrap();

        write!(f, "clear_mini ").unwrap();

        for i in 0..4 {
            write!(
                f,
                "{} ",
                Difference(
                    self.active.clear_mini[i as usize],
                    rhs.active.clear_mini[i as usize]
                )
            )
            .unwrap();
        }

        writeln!(f).unwrap();

        write!(f, "clear_spin ").unwrap();

        for i in 0..4 {
            write!(
                f,
                "{} ",
                Difference(
                    self.active.clear_spin[i as usize],
                    rhs.active.clear_spin[i as usize]
                )
            )
            .unwrap();
        }

        writeln!(f).unwrap();

        writeln!(
            f,
            "b2b:combo {} {}",
            Difference(self.active.b2b, rhs.active.b2b),
            Difference(self.active.combo, rhs.active.combo)
        )
        .unwrap();
        writeln!(
            f,
            "perfect_clear {}",
            Difference(self.active.perfect_clear, rhs.active.perfect_clear)
        )
        .unwrap();

        f
    }
}
