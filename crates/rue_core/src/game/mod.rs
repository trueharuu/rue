//! Singleplayer and multiplayer game logic. Contains rulesets, game state and attack resolution.

pub mod attack;
pub mod ruleset;
pub mod garbage;

use crate::game::attack::{AttackContext, Clear, b2b_chaining_bonus};
use crate::game::garbage::GarbageQueue;
use crate::header::WIDTH;
use crate::rng::Rng;
use crate::{board::Board, game::ruleset::Ruleset, piece::Piece, placement::Move, spin::Spin};

#[derive(Clone)]
/// A full singleplayer game state.
pub struct Game<const N: usize> {
    /// The full board.
    pub board: Board<N>,
    /// The piece occupying the hold slot, if any.
    pub hold: Option<Piece>,
    /// The queue of next places. The active piece is always `queue[0]`.
    pub queue: Vec<Piece>,
    /// The garbage queue, which is a FIFO of incoming garbage lines.
    pub garbage_queue: GarbageQueue,
    /// The current back-to-back count, if any.
    pub b2b_count: Option<u32>,
    /// The current combo count, if any.
    pub combo_count: Option<u32>,
    /// The current ruleset.
    pub ruleset: Ruleset,
    /// The RNG state.
    pub rng: Rng,
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
        let pre_combo = self.combo_count;

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
            // tank garbage if any
            let segments = self.garbage_queue.tank(self.ruleset.garbage_cap);
            
            for segment in segments {
                let col = self.rng.next() as u32 % WIDTH as u32;
                self.board.insert_garbage(segment, col);
            }

            return AttackContext {
                clear_type: Clear::None,
                spin_type: Spin::None,
                lines_cleared: 0,
                attack_sent: 0.0,
                b2b_before: pre_b2b.unwrap_or(0) as u8,
                b2b_after: self.b2b_count.unwrap_or(0) as u8,
                combo_before: pre_combo.unwrap_or(0),
                combo_after: 0,
                is_surge_release: false,
                is_garbage_clear: false,
                is_perfect_clear: false,
                piece: placement.piece(),
                placement,
            };
        }

        let clear_type = match line_clears {
            1 => Clear::Single,
            2 => Clear::Double,
            3 => Clear::Triple,
            4 => Clear::Quad,
            5 => Clear::Penta,
            _ => Clear::None,
        };

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

        let is_surge_release = surge_event > 0.0;

        let mut attack_sent = (main_event + surge_event + pc_event) as f32;

        // cancel garbage if any
        let garbage_canceled = self.garbage_queue.tank(attack_sent as u32);
        for segment in garbage_canceled {
            attack_sent -= segment as f32;
        }

        AttackContext {
            clear_type,
            spin_type: placement.spin(),
            lines_cleared: line_clears as u8,
            attack_sent,
            b2b_before: pre_b2b.unwrap_or(0) as u8,
            b2b_after: self.b2b_count.unwrap_or(0) as u8,
            combo_before: pre_combo.unwrap_or(0),
            combo_after: self.combo_count.unwrap_or(0),
            is_surge_release,
            is_garbage_clear: false,
            is_perfect_clear: is_pc,
            piece: placement.piece(),
            placement,
        }
    }
}
