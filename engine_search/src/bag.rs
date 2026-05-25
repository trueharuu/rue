use engine_core::{piece::{ALL_PIECES, PIECE_NB, Piece}, queue::Queue};

#[derive(Debug, Clone)]
pub(crate) struct BagTracker {
    seen: [bool; PIECE_NB],
    count: u8,
}

impl BagTracker {
    pub(crate) fn new() -> Self {
        Self {
            seen: [false; PIECE_NB],
            count: 0,
        }
    }

    pub(crate) fn consume(&mut self, piece: Piece) {
        if self.count >= 7 {
            self.reset();
        }
        let idx = piece as usize;
        if self.seen[idx] {
            self.reset();
        }
        self.seen[idx] = true;
        self.count += 1;
    }

    pub(crate) fn remaining(&self) -> Vec<Piece> {
        ALL_PIECES
            .iter()
            .copied()
            .filter(|&p| !self.seen[p as usize])
            .collect()
    }

    fn reset(&mut self) {
        self.seen = [false; PIECE_NB];
        self.count = 0;
    }
}

impl Default for BagTracker {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn extend_queue(queue: &Queue, current: Piece, hold: Option<Piece>) -> Queue {
    let mut tracker = BagTracker::new();

    if let Some(h) = hold {
        tracker.consume(h);
    }
    tracker.consume(current);

    for piece in queue {
        tracker.consume(piece);
    }

    let remaining = tracker.remaining();
    let mut extended = *queue;

    if remaining.len() <= 2 && !remaining.is_empty() {
        extended.extend(remaining);
    }

    extended
}
