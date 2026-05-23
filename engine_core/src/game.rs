use crate::{
    board::Board,
    data::SPAWN_COL,
    piece::Piece,
    placement::Move,
    ruleset::{self, ACTIVE_RULES, AttackConfig, AttackContext},
    spin::SpinType,
};

#[derive(Clone)]
pub struct Game {
    pub board: Board,
    pub current: Piece,
    pub hold: Option<Piece>,
    pub queue: Vec<Piece>,
    pub b2b: u8,
    pub combo: u32,
    pub pending_garbage: u8,
    pub config: AttackConfig,
}

impl Game {
    pub fn new(board: Board, current: Piece, queue: Vec<Piece>, config: AttackConfig) -> Self {
        Self {
            board,
            current,
            hold: None,
            queue,
            b2b: 0,
            combo: 0,
            pending_garbage: 0,
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
        let ctx = AttackContext {
            lines: lc as u8,
            b2b: self.b2b,
            combo: self.combo as u8,
            spin: m.spin(),
            b2b_broken_from: None,
            clears_garbage: false,
            config: self.config,
            is_perfect_clear: self.board.is_empty(),
        };
        let mut outgoing = ruleset::calculate_attack(
            lc as u8,
            m.spin(),
            self.b2b,
            self.combo as u8,
            self.config,
            self.board.is_empty(),
        ) as u8;

        if self.pending_garbage > 0 {
            // 3 cases
            // outgoing > incoming, send difference and reset pending
            // outgoing == incoming, both 0
            // outgoing < incoming, send 0 and reduce pending by outgoing
            if outgoing >= self.pending_garbage {
                self.pending_garbage = 0;
                outgoing -= self.pending_garbage;
            } else if outgoing < self.pending_garbage {
                self.pending_garbage -= outgoing;
                outgoing = 0;
            } else {
                outgoing = 0;
                self.pending_garbage = 0;
            }
        }

        if lc > 0 {
            let next_b2b = if m.spin() != SpinType::NoSpin || lc == 4 {
                self.b2b.saturating_add(1)
            } else {
                0
            };
            let next_combo = self.combo.saturating_add(1);

            self.b2b = next_b2b;
            self.combo = next_combo;
        } else {
            const CAP: u8 = 8;
            let removed = self.pending_garbage.min(CAP);
            self.pending_garbage -= removed;

            if removed > 0 {
                self.board
                    .spawn_garbage(removed as i32, rand::random_range(0..10));
            }
        }

        let should_hold = self.infer_hold_used_for_piece(m.piece());
        if should_hold {
            if self.hold.is_some() {
                self.hold = Some(self.current);
                self.current = self.queue.remove(0);
            } else {
                self.hold = Some(self.current);
                self.queue.remove(0);
                self.current = self.queue.remove(0);
            }
        } else {
            self.current = self.queue.remove(0);
        }

        (ctx, outgoing)
    }
}
