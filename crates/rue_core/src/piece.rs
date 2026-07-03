//! Tetromino identifiers and per-piece geometric helpers.

use std::{fmt::Display, marker::ConstParamTy};

/// Tetromino kind encoded as a compact integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ConstParamTy)]
pub enum Piece {
    /// T tetromino.
    T = 0,
    /// I tetromino.
    I = 1,
    /// J tetromino.
    J = 2,
    /// L tetromino.
    L = 3,
    /// O tetromino.
    O = 4,
    /// S tetromino.
    S = 5,
    /// Z tetromino.
    Z = 6,
}

impl Piece {
    /// Number of tetromino kinds.
    pub const NB: usize = 7;
    /// All tetromino kinds in canonical index order.
    pub const ALL: [Piece; Self::NB] = [
        Piece::T,
        Piece::I,
        Piece::J,
        Piece::L,
        Piece::O,
        Piece::S,
        Piece::Z,
    ];

    #[must_use]
    /// Converts a compact integer to a piece, returning `None` when out of range.
    pub const fn from_u8(n: u8) -> Option<Piece> {
        match n {
            0 => Some(Piece::T),
            1 => Some(Piece::I),
            2 => Some(Piece::J),
            3 => Some(Piece::L),
            4 => Some(Piece::O),
            5 => Some(Piece::S),
            6 => Some(Piece::Z),
            _ => None,
        }
    }

    #[must_use]
    /// Returns `true` for pieces with 180-degree rotational symmetry groups.
    pub const fn group2(self) -> bool {
        matches!(self, Piece::I | Piece::S | Piece::Z)
    }

    #[must_use]
    /// Returns `true` for pieces with full 4-state rotational symmetry groups.
    pub const fn group3(self) -> bool {
        matches!(self, Piece::T | Piece::L | Piece::J)
    }

    #[must_use]
    /// Number of distinct canonical rotations for this piece.
    pub const fn canonical_rotations(self) -> usize {
        if matches!(self, Piece::O) {
            1
        } else if self.group2() {
            2
        } else {
            4
        }
    }

    #[must_use]
    /// Number of rotation states explored by search for this piece.
    pub const fn search_size(self) -> usize {
        if matches!(self, Piece::O) { 1 } else { 4 }
    }

    #[must_use]
    /// Maps an arbitrary rotation index to its canonical representative.
    pub const fn canonical_rotation(self, r: usize) -> usize {
        if matches!(self, Piece::O) {
            0
        } else if self.group2() {
            r & 1
        } else {
            r
        }
    }

    #[must_use]
    /// Returns translation offsets needed to align canonical rotation frames.
    pub const fn canonical_offset(self, r: usize) -> (i32, i32) {
        if matches!(self, Piece::I) {
            if r == 2 {
                return (1, 0);
            }
            if r == 3 {
                return (0, -1);
            }
        }
        if matches!(self, Piece::S | Piece::Z) {
            if r == 2 {
                return (0, 1);
            }
            if r == 3 {
                return (1, 0);
            }
        }
        (0, 0)
    }

    #[must_use]
    /// Generation height adjustment used by placement generation.
    pub const fn h_gen(self) -> i32 {
        if matches!(self, Piece::I | Piece::T) {
            2
        } else if matches!(self, Piece::O) {
            0
        } else {
            1
        }
    }

    #[must_use]
    /// Spawn height adjustment used by spawn placement logic.
    pub const fn h_spawn(self) -> i32 {
        if matches!(self, Piece::I) {
            2
        } else if matches!(self, Piece::O) {
            0
        } else {
            1
        }
    }

    #[must_use]
    /// Placement height adjustment used by grounded placement logic.
    pub const fn h_place(self) -> i32 {
        2 + (matches!(self, Piece::I) as i32) - (matches!(self, Piece::O) as i32)
    }

    #[must_use]
    /// Three non-origin mino offsets for the spawn orientation.
    pub const fn base_cells(self) -> [(i8, i8); 3] {
        match self {
            Piece::I => [(-1, 0), (1, 0), (2, 0)],
            Piece::O => [(1, 0), (0, 1), (1, 1)],
            Piece::T => [(-1, 0), (1, 0), (0, 1)],
            Piece::L => [(-1, 0), (1, 0), (1, 1)],
            Piece::J => [(-1, 0), (1, 0), (-1, 1)],
            Piece::S => [(-1, 0), (0, 1), (1, 1)],
            Piece::Z => [(-1, 1), (0, 1), (1, 0)],
        }
    }
}

impl Display for Piece {
    /// Formats the piece as its single-letter tetromino symbol.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Piece::T => write!(f, "T"),
            Piece::I => write!(f, "I"),
            Piece::J => write!(f, "J"),
            Piece::L => write!(f, "L"),
            Piece::O => write!(f, "O"),
            Piece::S => write!(f, "S"),
            Piece::Z => write!(f, "Z"),
        }
    }
}