pub mod ruleset;
pub mod garbage;
pub mod attack;

use crate::board::Board;
use crate::buffer::Buffer;
use crate::header::WIDTH;
use crate::piece::Piece;
use crate::placement::Move;
use crate::rng::Rng;
use crate::rule::Rule;
use crate::spin::Spin;
use crate::game::ruleset::Ruleset;
use crate::game::garbage::GarbageQueue;
use crate::game::attack::Attack;
use crate::game::attack::compute_attack;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Game<const N: usize, const RULE: Rule> {
    pub rng: Rng,
    pub grng: Rng,
    pub board: Board<N>,
    pub queue: Buffer<Piece, 28>,
    pub hold: Option<Piece>,
    pub combo: Option<u32>,
    pub b2b: Option<u32>,
    pub garbage_queue: GarbageQueue,
    pub ruleset: Ruleset,
}

impl<const N: usize, const RULE: Rule> Game<N, RULE> {
    /// Applies `placement` and returns the attack it produces.
    ///
    /// Consumes queue and hold exactly as `play` does, then tanks or cancels
    /// garbage with the RNG and garbage queue.
    pub fn advance(&mut self, placement: &Move) -> Attack {
        let mut attack = self.play(*placement);

        if attack.line_clears == 0 {
            let segments = self.garbage_queue.tank(self.ruleset.garbage_cap);

            for segment in segments {
                let col = self.grng.next() as u32 % WIDTH as u32;
                self.board.insert_garbage(segment, col);
            }

            return attack;
        }

        // cancel garbage if any
        let garbage_canceled = self.garbage_queue.tank(attack.total);
        attack.canceled = garbage_canceled.iter().sum();

        attack
    }

    /// Applies `placement` without garbage handling and returns the attack it
    /// produces.
    ///
    /// Updates the board, queue, hold, combo, and b2b exactly as `advance`
    /// does. The search uses this to simulate children without an RNG or
    /// garbage queue.
    pub fn play(&mut self, placement: Move) -> Attack {
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

            if is_special_clear || (is_pc && self.ruleset.pc_b2b.is_some()) {
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
            &self.ruleset,
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

