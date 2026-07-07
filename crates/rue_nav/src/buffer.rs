//! Move result buffers grouped by rotation and spin outcome.

use rue_core::{
    board::Board, header::WIDTH, piece::Piece, placement::Move, rotation::Rotation, spin::Spin,
};

/// Result of move generation: landable positions per rotation, per spin type.
///
/// Layout:
/// - `none[r]`: positions reachable without a final rotation
/// - `mini[r]`: positions with a mini-spin (or immobility spin)
/// - `full[r]`: positions with a full spin
#[derive(Clone, Copy)]
pub struct Moves<const N: usize> {
    /// The piece that these moves are for.
    pub piece: Piece,
    /// Landable positions without a final spin classification.
    pub none: [Board<N>; 4],
    /// Landable positions classified as mini spins.
    pub mini: [Board<N>; 4],
    /// Landable positions classified as full spins.
    pub full: [Board<N>; 4],
}

impl<const N: usize> Moves<N> {
    /// Empty move buffer with no landable cells in any bucket.

    pub const fn empty(piece: Piece) -> Self {
        Self {
            piece,
            none: [Board::EMPTY; 4],
            mini: [Board::EMPTY; 4],
            full: [Board::EMPTY; 4],
        }
    }

    #[inline]
    #[must_use]
    /// Returns the board bucket for the given `spin` and `rotation` index.
    pub fn get(&self, spin: Spin, rotation: usize) -> Board<N> {
        match spin {
            Spin::None => self.none[rotation],
            Spin::Mini => self.mini[rotation],
            Spin::Full => self.full[rotation],
        }
    }

    #[inline]
    /// Returns a mutable board bucket for the given `spin` and `rotation` index.
    pub fn get_mut(&mut self, spin: Spin, rotation: usize) -> &mut Board<N> {
        match spin {
            Spin::None => &mut self.none[rotation],
            Spin::Mini => &mut self.mini[rotation],
            Spin::Full => &mut self.full[rotation],
        }
    }

    /// Iterates all occupied move cells as packed `Move` values for piece `P`.
    pub fn iter(&self) -> impl Iterator<Item = Move> {
        (0..4).flat_map(move |r| {
            [Spin::None, Spin::Mini, Spin::Full]
                .into_iter()
                .flat_map(move |s| {
                    let b = self.get(s, r);
                    (0..Board::<N>::H).flat_map(move |y| {
                        (0..WIDTH).filter_map(move |x| {
                            if b.get(x, y) {
                                Some(Move::new(self.piece, Rotation::from(r as u8), x, y, s))
                            } else {
                                None
                            }
                        })
                    })
                })
        })
    }

    #[inline]
    #[must_use]
    /// Counts all occupied move cells across all rotations and spin buckets.
    pub fn count(&self) -> u32 {
        let mut total = 0;
        for r in 0..4 {
            total += self.none[r].popcount();
            total += self.mini[r].popcount();
            total += self.full[r].popcount();
        }
        total
    }
}
