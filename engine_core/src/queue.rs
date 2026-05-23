use crate::piece::{ALL_PIECES, PIECE_NB, Piece};

#[derive(Debug, Clone)]
pub struct BagTracker {
    seen: [bool; PIECE_NB],
    count: u8,
}

impl BagTracker {
    /// Create a new tracker with an empty bag (no pieces consumed).
    pub fn new() -> Self {
        Self {
            seen: [false; PIECE_NB],
            count: 0,
        }
    }

    /// Mark a piece as consumed in the current bag.
    ///
    /// If the bag is complete (all 7 consumed), resets to a new bag and
    /// marks the piece as the first of that new bag.
    pub fn consume(&mut self, piece: Piece) {
        if self.count >= 7 {
            self.reset();
        }
        let idx = piece as usize;
        // If already seen, we've crossed a bag boundary — reset first.
        if self.seen[idx] {
            self.reset();
        }
        self.seen[idx] = true;
        self.count += 1;
    }

    pub fn remaining(&self) -> Vec<Piece> {
        ALL_PIECES
            .iter()
            .copied()
            .filter(|&p| !self.seen[p as usize])
            .collect()
    }

    #[allow(dead_code)]
    pub fn predict_next(&mut self, queue: &[Piece]) -> Vec<Piece> {
        for &piece in queue {
            self.consume(piece);
        }
        self.remaining()
    }

    #[allow(dead_code)]
    pub fn count(&self) -> u8 {
        self.count
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

pub fn extend_queue(queue: &[Piece], current: Piece, hold: Option<Piece>) -> Vec<Piece> {
    let mut tracker = BagTracker::new();

    if let Some(h) = hold {
        tracker.consume(h);
    }

    tracker.consume(current);

    for &piece in queue {
        tracker.consume(piece);
    }

    let remaining = tracker.remaining();
    let mut extended = queue.to_vec();

    // Only predict when ≤2 pieces remain — those are guaranteed to appear
    // before the next bag, though their order is unknown.
    if remaining.len() <= 2 && !remaining.is_empty() {
        extended.extend_from_slice(&remaining);
    }

    extended
}