//! Move buffers produced by move generation.

use rue_core::board::Board;
use rue_core::header::WIDTH;
use rue_core::piece::Piece;
use rue_core::placement::Move;
use rue_core::rotation::Rotation;
use rue_core::spin::Spin;

/// Result of move generation. Contains reachable landed positions per rotation, per
/// spin-type.
///
/// Layout:
/// - `none[r]`: positions reachable with no spin
/// - `mini[r]`: positions reachable with a spin-mini
/// - `full[r]`: positions reachable with a spin-full
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moves<const N: usize> {
    /// The piece for which these moves are for.
    pub piece: Piece,
    /// Landable positions without a final spin classification.
    pub none: [Board<N>; Rotation::NB],
    /// Landable positions classified as mini spins.
    pub mini: [Board<N>; Rotation::NB],
    /// Landable positions classified as full spins.
    pub full: [Board<N>; Rotation::NB],
}

impl<const N: usize> Moves<N> {
    /// Creates a new empty `Moves` result for the given piece.
    #[inline]
    #[must_use]
    pub const fn empty(piece: Piece) -> Self {
        Self {
            piece,
            none: [Board::empty(); Rotation::NB],
            mini: [Board::empty(); Rotation::NB],
            full: [Board::empty(); Rotation::NB],
        }
    }

    /// Inserts a new [`Move`] into the buffer.
    /// Returns `true` if the move was not already present, `false` otherwise.
    #[inline]
    #[must_use]
    pub fn insert(&mut self, mv: Move) -> bool {
        let r = mv.rotation() as usize;
        let board = match mv.spin() {
            Spin::None => &mut self.none[r],
            Spin::Mini => &mut self.mini[r],
            Spin::Full => &mut self.full[r],
        };

        if board.get(mv.x(), mv.y()) {
            return false;
        }

        board.set(mv.x(), mv.y());
        true
    }

    /// Returns `true` if the buffer contains the given [`Move`], `false` otherwise.
    #[inline]
    #[must_use]
    pub fn contains(&self, mv: Move) -> bool {
        let r = mv.rotation() as usize;
        let board = match mv.spin() {
            Spin::None => &self.none[r],
            Spin::Mini => &self.mini[r],
            Spin::Full => &self.full[r],
        };

        board.get(mv.x(), mv.y())
    }

    /// Returns an iterator over all moves stored in this buffer, in ascending
    /// `(rotation, spin, y, x)` order.
    #[inline]
    #[must_use]
    pub const fn iter(&self) -> MovesIter<'_, N> {
        MovesIter {
            moves: self,
            rotation: 0,
            spin: 0,
            x: 0,
            y: 0,
        }
    }

    /// Returns the total number of moves in the buffer.
    #[inline]
    #[must_use]
    pub fn popcount(&self) -> u64 {
        let mut total = 0;
        for r in 0..Rotation::NB {
            total += self.none[r].popcount();
            total += self.mini[r].popcount();
            total += self.full[r].popcount();
        }

        total
    }
}

/// An iterator over the moves in a [`Moves`] buffer.
pub struct MovesIter<'a, const N: usize> {
    moves: &'a Moves<N>,
    rotation: usize,
    spin: usize,
    x: i32,
    y: i32,
}

impl<const N: usize> Iterator for MovesIter<'_, N> {
    type Item = Move;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.rotation < Rotation::NB {
            let board = match self.spin {
                0 => &self.moves.none[self.rotation],
                1 => &self.moves.mini[self.rotation],
                2 => &self.moves.full[self.rotation],
                _ => unreachable!(),
            };

            while self.y < Board::<N>::total_height() {
                while self.x < WIDTH {
                    if board.get(self.x, self.y) {
                        let mv = Move::new(
                            self.moves.piece,
                            self.x,
                            self.y,
                            Rotation::from_u8(self.rotation as u8),
                            match self.spin {
                                0 => Spin::None,
                                1 => Spin::Mini,
                                2 => Spin::Full,
                                _ => unreachable!(),
                            },
                        );
                        self.x += 1;
                        return Some(mv);
                    }
                    self.x += 1;
                }
                self.x = 0;
                self.y += 1;
            }
            self.y = 0;
            self.spin += 1;
            if self.spin > 2 {
                self.spin = 0;
                self.rotation += 1;
            }
        }
        None
    }
}

impl<'a, const N: usize> IntoIterator for &'a Moves<N> {
    type Item = Move;
    type IntoIter = MovesIter<'a, N>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
