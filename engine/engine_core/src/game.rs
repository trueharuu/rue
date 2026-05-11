use crate::{
    board::Board, data::SPAWN_COL, piece::Piece, placement::Move, ruleset::ACTIVE_RULES,
    spin::SpinType,
};

#[derive(Clone)]
pub struct GameState {
    pub board: Board,
    pub current: Piece,
    pub hold: Option<Piece>,
    pub queue: Vec<Piece>,
    pub b2b: u8,
    pub combo: u32,
    pub pending_garbage: u8,
    pub lines_total: u32,
    pub bag_number: u32,
    pub pieces_into_bag: u8,
}

impl GameState {
    pub fn new(board: Board, current: Piece, queue: Vec<Piece>) -> Self {
        Self {
            board,
            current,
            hold: None,
            queue,
            b2b: 0,
            combo: 0,
            pending_garbage: 0,
            lines_total: 0,
            bag_number: 0,
            pieces_into_bag: 0,
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

    pub fn advance(&mut self, m: &Move) {
        let lc = self.board.do_move(m);

        if lc > 0 {
            let next_b2b = if m.spin() != SpinType::NoSpin || lc == 4 {
                self.b2b.saturating_add(1)
            } else {
                0
            };
            let next_combo = self.combo.saturating_add(1);

            self.b2b = next_b2b;
            self.combo = next_combo;
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
    }
}