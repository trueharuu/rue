use rue_core::{game::Game, placement::Move, ruleset::AttackContext};
use serde::{Deserialize, Serialize};

use crate::{active::ActiveModel, board::BoardModel};

pub mod active;
pub mod board;
pub mod diff;
pub mod features;
pub mod opponent;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Model {
    pub board: BoardModel,
    pub active: ActiveModel,
    // pub opponent: OpponentModel,
    pub board_weight: f64,
    pub active_weight: f64,
}

impl Model {
    #[must_use]
    pub fn eval(&self, game: &Game, active: &Move, ctx: &AttackContext) -> f64 {
        self.board_weight * self.board.eval(game)
            + self.active_weight * self.active.eval(active, ctx)
        // + opponent.map_or(0.0, |x| self.opponent.eval(x))
    }
}
