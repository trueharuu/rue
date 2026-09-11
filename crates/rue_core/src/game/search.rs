//! Compact simulation state for the beam search.

use crate::board::Board;
use crate::buffer::Buffer;
use crate::game::attack::Attack;
use crate::game::attack::compute_attack;
use crate::game::ruleset::Ruleset;
use crate::piece::Piece;
use crate::placement::Move;
use crate::rule::Rule;
use crate::spin::Spin;

use super::Game;

/// The parts of a game that a placement can change.
///
/// The search stores one of these per node instead of a full [`Game`]: the
/// RNGs, ruleset, and garbage queue are never modified during a search, so
/// they are not part of the search state. `incoming` is the total garbage
/// owed at the time the state was created; it is constant for the whole
/// search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchGame<const N: usize> {
    pub board: Board<N>,
    pub queue: Buffer<Piece, 28>,
    pub hold: Option<Piece>,
    pub combo: Option<u32>,
    pub b2b: Option<u32>,
    pub incoming: u32,
}

impl<const N: usize> SearchGame<N> {
    /// Applies `placement` without garbage handling and returns the attack it
    /// produces.
    ///
    /// Updates the board, queue, hold, combo, and b2b exactly as
    /// [`Game::advance`] does for a game with no garbage.
    pub fn play(&mut self, placement: Move, ruleset: &Ruleset) -> Attack {
        let requires_hold = placement.piece() != self.queue[0];
        let line_clears = self.board.do_move(placement) as u32;

        {
            let has_held = self.hold.is_some();

            if !has_held && !requires_hold {
                self.queue.remove(0);
            } else if !has_held && requires_hold {
                self.hold = Some(self.queue[0]);
                self.queue.remove(0);
                self.queue.remove(0);
            } else if has_held && requires_hold {
                self.hold = Some(self.queue[0]);
                self.queue.remove(0);
            } else if has_held && !requires_hold {
                self.queue.remove(0);
            }
        }

        let is_special_clear = placement.spin() != Spin::None || line_clears >= 4;
        let is_pc = self.board == Board::<N>::empty() && line_clears > 0;
        let pre_b2b = self.b2b;
        let pre_combo = self.combo;

        if line_clears > 0 {
            match self.combo {
                Some(c) => self.combo = Some(c + 1),
                None => self.combo = Some(0),
            }

            if is_special_clear || (is_pc && ruleset.pc_b2b.is_some()) {
                match self.b2b {
                    Some(b) => self.b2b = Some(b + 1),
                    None => self.b2b = Some(0),
                }
            } else {
                self.b2b = None;
            }
        } else {
            self.combo = None;
        }

        compute_attack(
            ruleset,
            placement.spin(),
            line_clears,
            is_pc,
            self.b2b,
            self.combo,
            pre_b2b,
            pre_combo,
            0,
        )
    }
}

impl<const N: usize, const RULE: Rule> From<&Game<N, RULE>> for SearchGame<N> {
    fn from(game: &Game<N, RULE>) -> Self {
        Self {
            board: game.board,
            queue: game.queue,
            hold: game.hold,
            combo: game.combo,
            b2b: game.b2b,
            incoming: game.garbage_queue.total(),
        }
    }
}