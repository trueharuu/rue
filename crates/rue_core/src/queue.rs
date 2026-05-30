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

pub const MAX_QUEUE: usize = 64;
#[derive(Debug, Clone, Copy)]
pub struct Queue([Option<Piece>; MAX_QUEUE]);

impl Queue {
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        let mut count = 0;
        while count < MAX_QUEUE && self.0[count].is_some() {
            count += 1;
        }

        count
    }

    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0[0].is_none()
    }

    #[inline]
    #[must_use]
    pub const fn get(&self, index: usize) -> Option<&Piece> {
        if index < MAX_QUEUE {
            self.0[index].as_ref()
        } else {
            None
        }
    }

    #[inline]
    #[must_use]
    pub const fn first(&self) -> Option<&Piece> {
        self.get(0)
    }

    #[inline]
    pub const fn remove_first(&mut self) -> Piece {
        let prev = self.0[0];
        let mut i = 0;
        while i < MAX_QUEUE - 1 {
            self.0[i] = self.0[i + 1];
            i += 1;
        }

        self.0[MAX_QUEUE - 1] = None;

        prev.unwrap()
    }

    #[inline]
    #[must_use]
    pub const fn from_slice(slice: &[Piece]) -> Self {
        let mut arr = [None; MAX_QUEUE];
        let mut i = 0;
        while i < slice.len() && i < MAX_QUEUE {
            arr[i] = Some(slice[i]);
            i += 1;
        }
        Self(arr)
    }

    pub const fn slice(&self, start: usize, end: usize) -> Self {
        let mut arr = [None; MAX_QUEUE];
        let mut i = 0;
        while i < end - start && start + i < MAX_QUEUE {
            arr[i] = self.0[start + i];
            i += 1;
        }
        Self(arr)
    }
}

pub struct QueueIter<'a> {
    queue: &'a Queue,
    index: usize,
}

impl<'a> Iterator for QueueIter<'a> {
    type Item = Piece;

    fn next(&mut self) -> Option<Self::Item> {
        let piece = self.queue.get(self.index)?;
        self.index += 1;
        Some(*piece)
    }
}

impl<'a> IntoIterator for &'a Queue {
    type Item = Piece;
    type IntoIter = QueueIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        QueueIter {
            queue: self,
            index: 0,
        }
    }
}

impl Extend<Piece> for Queue {
    fn extend<T: IntoIterator<Item = Piece>>(&mut self, iter: T) {
        let mut i = self.len();
        for piece in iter {
            if i < MAX_QUEUE {
                self.0[i] = Some(piece);
                i += 1;
            } else {
                break;
            }
        }
    }
}
