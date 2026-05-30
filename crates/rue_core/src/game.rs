use crate::{
    board::Board, data::SPAWN_COL, garbage::GarbageQueue, piece::Piece, placement::Move, queue::Queue, ruleset::{self, ACTIVE_RULES, AttackConfig, AttackContext}, spin::SpinType
};

#[derive(Clone, Copy, Debug)]
pub struct Game {
    pub board: Board,
    pub current: Piece,
    pub hold: Option<Piece>,
    pub queue: Queue,
    pub b2b: u8,
    pub combo: u32,
    pub pending_garbage: GarbageQueue,
    pub config: AttackConfig,
}

impl Game {
    pub fn new(board: Board, current: Piece, queue: Queue, config: AttackConfig) -> Self {
        Self {
            board,
            current,
            hold: None,
            queue,
            b2b: 0,
            combo: 0,
            pending_garbage: GarbageQueue::new(),
            config,
        }
    }

    /// next piece from queue, or None if exhausted
    pub fn queue_piece(&self, index: usize) -> Option<Piece> {
        self.queue.get(index).copied()
    }

    /// how many pieces remain in queue
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn infer_hold_used_for_piece(&self, piece: Piece) -> bool {
        if self.hold == Some(piece) {
            return true;
        }
        self.hold.is_none() && self.queue.first().copied() == Some(piece) && piece != self.current
    }

    pub fn spawn_envelope_blocked(board: &Board) -> bool {
        let spawn_y = ACTIVE_RULES.spawn_row;
        if spawn_y < 0 {
            return false;
        }

        let pivot_x = SPAWN_COL as i32;
        let envelope = [
            (pivot_x - 1, spawn_y),
            (pivot_x, spawn_y),
            (pivot_x + 1, spawn_y),
            (pivot_x + 2, spawn_y),
            (pivot_x - 1, spawn_y + 1),
            (pivot_x, spawn_y + 1),
            (pivot_x + 1, spawn_y + 1),
        ];

        envelope
            .iter()
            .any(|(x, y)| board.obstructed(*x, *y) || board.occupied(*x, *y))
    }

    pub fn advance(&mut self, m: &Move) -> (AttackContext, u8) {
        let lc = self.board.do_move(m);
        let is_b2b_eligible = m.spin() != SpinType::None || lc >= 4;
        let b2b_broken_from = if self.config.b2b_charging && lc > 0 && !is_b2b_eligible {
            Some(self.b2b)
        } else {
            None
        };
        let lines = lc as u8;
        let b2b = self.b2b;
        let combo = self.combo as u8;
        let spin = m.spin();
        let config = self.config;
        let is_perfect_clear = self.board.is_empty();
        let clears_garbage = false;

        let ctx = AttackContext {
            lines,
            b2b,
            combo,
            spin,
            b2b_broken_from,
            clears_garbage,
            config,
            is_perfect_clear,
        };
        let b2b_for_attack = if is_b2b_eligible { b2b } else { 0 };
        let calc_ctx = AttackContext {
            lines,
            b2b: b2b_for_attack,
            combo,
            spin,
            b2b_broken_from,
            clears_garbage,
            config,
            is_perfect_clear,
        };
        let mut outgoing = ruleset::calculate_attack_full(calc_ctx) as u8;

        if self.pending_garbage.total() > 0 {
            // 3 cases
            // outgoing > incoming, send difference and reset pending
            // outgoing == incoming, both 0
            // outgoing < incoming, send 0 and reduce pending by outgoing
            // if outgoing as usize >= self.pending_garbage.total() {
            //     self.pending_garbage.clear();
            //     outgoing -= self.pending_garbage;
            // } else if outgoing < self.pending_garbage {
            //     self.pending_garbage -= outgoing;
            //     outgoing = 0;
            // } else {
            //     outgoing = 0;
            //     self.pending_garbage = 0;
            // }
            outgoing = self.pending_garbage.remove(outgoing as usize) as u8;
        }

        if lc > 0 {
            let next_b2b = if is_b2b_eligible {
                self.b2b.saturating_add(1)
            } else {
                0
            };
            let next_combo = self.combo.saturating_add(1);

            self.b2b = next_b2b;
            self.combo = next_combo;
        } else {
            const CAP: usize = 8;
            let removed = self.pending_garbage.total().min(CAP);
            let t = self.pending_garbage.split(removed);

            if t.total() > 0 {
                for segment in t.segments {
                    if segment == 0 {
                        break;
                    }
                    self.board.spawn_garbage(segment as i32, rand::random_range(0..10));
                }
            }

            self.combo = 0;
        }

        let should_hold = self.infer_hold_used_for_piece(m.piece());
        if should_hold {
            if self.hold.is_some() {
                self.hold = Some(self.current);
                self.current = self.queue.remove_first();
            } else {
                self.hold = Some(self.current);
                self.queue.remove_first();
                self.current = self.queue.remove_first();
            }
        } else {
            self.current = self.queue.remove_first();
        }

        (ctx, outgoing)
    }
}
