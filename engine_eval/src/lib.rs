use engine_core::{game::Game, placement::Move};

use crate::{active::ActiveModel, board::BoardModel};

pub mod board;
pub mod features;
pub mod active;
pub mod opponent;

#[derive(Clone, Copy)]
pub struct Model {
    pub board: BoardModel,
    pub active: ActiveModel,
    // pub opponent: OpponentModel,

    pub w_board: f64,
    pub w_active: f64,
    // pub w_opponent: f64,
}

impl Model {
    #[must_use]
    pub fn eval(&self, game: &Game, active: &Move) -> f64 {
        self.w_board * self.board.eval(game) + self.w_active * self.active.eval(active)
    }
}