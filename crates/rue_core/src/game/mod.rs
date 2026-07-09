//! Singleplayer and multiplayer game logic. Contains rulesets, game state and attack resolution.

pub mod attack;
pub mod ruleset;

use crate::game::attack::{AttackContext, b2b_chaining_bonus};
use crate::{board::Board, game::ruleset::Ruleset, piece::Piece, placement::Move, spin::Spin};

#[derive(Clone)]
/// A full singleplayer game state.
pub struct Game<const N: usize> {
    /// The full board.
    pub board: Board<N>,
    /// The row within the board where the transition between garbage and our own stack occurs.
    pub garbage_row: u8,
    /// The piece occupying the hold slot, if any.
    pub hold: Option<Piece>,
    /// The queue of next places. The active piece is always `queue[0]`.
    pub queue: Vec<Piece>,
    /// The garbage queue, which is a FIFO of incoming garbage lines.
    /// The first element is the next group of lines to be added.
    /// We can safely assume that no singular attack will send more than [`u32::MAX`] at once.
    pub garbage_queue: Vec<u32>,
    /// The current back-to-back count, if any.
    pub b2b_count: Option<u32>,
    /// The current combo count, if any.
    pub combo_count: Option<u32>,
    /// The current ruleset.
    pub ruleset: Ruleset,
}

impl<const N: usize> Game<N> {
    /// The next placeable pieces.
    #[must_use]
    pub fn active(&self) -> (Piece, Piece) {
        if let Some(s) = self.hold {
            (self.queue[0], s)
        } else {
            (self.queue[0], self.queue[1])
        }
    }

    // TODO: garbage cancelling/tanking, implement garbage queue mechanics
    /// Advance the game state by one placement.
    /// Returns the total number of lines sent to the opponent, if any. Value is kept as a float to be rounded by the caller.
    pub fn tick(&mut self, placement: Move) -> AttackContext {
        let requires_hold = placement.piece() != self.queue[0];

        let line_clears = self.board.do_move(placement);

        // apply changes to queue and hold
        {
            let has_held = self.hold.is_some();

            // 4 cases
            // no piece held, we want to place Abcdefg
            if !has_held && !requires_hold {
                self.queue.remove(0);
            }
            // no piece held, we want to place aBcdefg
            else if !has_held && requires_hold {
                self.hold = Some(self.queue[0]);
                self.queue.remove(0);
                self.queue.remove(0);
            }
            // piece held, we want to place [A]bcdefg
            else if has_held && requires_hold {
                self.hold = Some(self.queue[0]);
                self.queue.remove(0);
            }
            // piece held, we want to place [a]Bcdefg
            else if has_held && !requires_hold {
                self.queue.remove(0);
            }
        }

        let is_special_clear = placement.spin() != Spin::None || line_clears >= 4;
        let is_pc = self.board == Board::<N>::EMPTY && line_clears > 0;
        let pre_b2b = self.b2b_count;

        if line_clears > 0 {
            match self.combo_count {
                Some(c) => self.combo_count = Some(c + 1),
                None => self.combo_count = Some(0),
            }

            if is_special_clear || (is_pc && self.ruleset.pc_b2b.is_some()) {
                match self.b2b_count {
                    Some(b) => self.b2b_count = Some(b + 1),
                    None => self.b2b_count = Some(0),
                }
            } else {
                self.b2b_count = None;
            }

            if let Some(b2b) = self.ruleset.pc_b2b
                && is_pc
            {
                match self.b2b_count {
                    Some(b) => self.b2b_count = Some(b + b2b),
                    None => self.b2b_count = Some(b2b),
                }
            }
        } else {
            self.combo_count = None;
        }

        if line_clears == 0 {
            return AttackContext {
                placement,
                line_clears: 0,
                is_pc: false,
                sent: 0.0,
                is_b2b: false,
                garbage_cancelled: 0.0,
                garbage_tanked: 0.0,
                outgoing: 0.0,
            };
        }

        let mut garbage = f64::from(self.ruleset.base_attack(line_clears, placement.spin()));

        if let Some(s) = self.b2b_count
            && s > 0
        {
            if self.ruleset.b2b_chaining {
                garbage += b2b_chaining_bonus(s, &self.ruleset);
            } else {
                garbage += f64::from(self.ruleset.back_to_back_bonus);
            }
        }

        if let Some(combo) = self.combo_count
            && combo > 0
        {
            garbage *= 1.0 + self.ruleset.combo_bonus * f64::from(combo);
            if combo > 1 {
                let combo_floor = (1.0 + f64::from(combo) * self.ruleset.combo_floor_scale).ln();
                garbage = garbage.max(combo_floor);
            }
        }

        let is_garbage_special = false;
        let special_bonus = if is_garbage_special && is_special_clear {
            f64::from(self.ruleset.garbage_clear_bonus)
        } else {
            0.0
        };

        let main_event = (garbage * self.ruleset.garbage_multiplier + special_bonus).floor();
        let chain_broken = !(is_special_clear || is_pc && self.ruleset.pc_b2b.is_some());
        let surge_event = if self.ruleset.b2b_charging {
            pre_b2b
                .filter(|_| chain_broken)
                .filter(|b| *b + 1 > self.ruleset.b2b_charging_start)
                .map_or(0.0, |b| {
                    (f64::from(
                        b - self.ruleset.b2b_charging_start + self.ruleset.back_to_back_bonus + 1,
                    ) * self.ruleset.garbage_multiplier)
                        .floor()
                        .max(0.0)
                })
        } else {
            0.0
        };
        let pc_event = if is_pc {
            (f64::from(self.ruleset.pc_garbage) * self.ruleset.garbage_multiplier).floor()
        } else {
            0.0
        };

        AttackContext {
            outgoing: main_event + surge_event + pc_event,
            is_pc,
            line_clears: line_clears as usize,
            placement,
            is_b2b: if self.ruleset.pc_b2b.is_some() {
                is_pc || is_special_clear
            } else {
                is_special_clear
            },
            garbage_cancelled: 0.0,
            garbage_tanked: 0.0,
            sent: 0.0,
        }
    }
}
