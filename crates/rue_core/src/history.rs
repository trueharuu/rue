//! Piece history and resource management.
//! This allows us to make assumptions about what the future could be, giving us a "likelihood" for when a certain piece comes.

use crate::piece::Piece;

/// The maximum number of pieces to track in the history.
/// 14 pieces is the worst-case wait for a single piece type in 7-bag.
/// After placing a piece, you must place all 6 others before it reappears.
/// The maximum wait is 13, `TXXXXXX|XXXXXXTX`. 14 captures the full range with a 1-element margin.
pub const CAPACITY: usize = 14;

/// Tracks up to the last [`CAPACITY`] placed pieces for piece recency features.
/// Seperate from [`Game`], and is reconstructed from the search path.
pub struct History {
    /// The buffer of pieces, with the most recent at the end.
    buf: [Piece; CAPACITY],
    /// The number of pieces currently in the buffer.
    /// This is monotonically increasing until it reaches [`CAPACITY`], after which it remains constant.
    len: usize,
}

impl History {
    /// Creates a new empty history.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            buf: [Piece::T; CAPACITY],
            len: 0,
        }
    }

    /// Record a placement into the buffer.
    pub fn push(&mut self, p: Piece) {
        if self.len < CAPACITY {
            self.buf[self.len] = p;
            self.len += 1;
        } else {
            // shift left and insert at the end
            self.buf.copy_within(1.., 0);
            self.buf[CAPACITY - 1] = p;
        }
    }

    /// Recency score per piece type.
    /// 0.0 = just placed (unlikely to reappear soon).
    /// 1.0 = not seen in window (likely in current bag, will appear soon).
    #[inline]
    #[must_use]
    pub fn recency(&self) -> [f32; 7] {
        let mut scores = [1.0f32; 7]; // default: not in window
        for i in 0..self.len {
            let piece_idx = self.buf[i] as usize;
            // Only overwrite if this is the first (most recent) occurrence
            if (scores[piece_idx] - 1.0).abs() < f32::EPSILON || scores[piece_idx] > i as f32 / CAPACITY as f32 {
                scores[piece_idx] = i as f32 / CAPACITY as f32;
            }
        }
        scores
    }
}
