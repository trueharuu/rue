//! Concrete piece placements.

use std::fmt::Debug;
use std::fmt::Display;

use crate::data::rot_cell;
use crate::header::WIDTH;
use crate::piece::Piece;
use crate::rotation::Rotation;
use crate::spin::Spin;

/// A single location and rotation of a piece.
/// A move has 5 fields:
/// - `piece`: one of [`Piece`]. Uses 3 bits.
/// - `x`: horizontal position in `0..10`. Uses 4 bits.
/// - `y`: vertical position in `0..64`. Uses 6 bits.
/// - `rotation`: one of [`Rotation`]. Uses 2 bits.
/// - `spin`: one of [`Spin`]. Uses 2 bits.
///
/// A single move packs into exactly 17 bits.
/// The most significant bits are always 0.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move(u32);

const PIECE_SHIFT: u32 = 0;
const X_SHIFT: u32 = 3;
const Y_SHIFT: u32 = 3 + 4;
const ROT_SHIFT: u32 = 3 + 4 + 6;
const SPIN_SHIFT: u32 = 3 + 4 + 6 + 2;
const PIECE_MASK: u32 = 0b111;
const X_MASK: u32 = 0b1111;
const Y_MASK: u32 = 0b11_1111;
const ROT_MASK: u32 = 0b11;
const SPIN_MASK: u32 = 0b11;

impl Move {
    /// Returns a null move. This is likely invalid.
    #[inline]
    #[must_use]
    pub const fn null() -> Self {
        Self(0)
    }

    /// Returns the raw 32-bit representation of the move.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Creates a [`Move`] from the raw 32-bit representation.
    ///
    /// # Safety
    /// The raw value must be a valid move.
    #[inline]
    #[must_use]
    pub const unsafe fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Creates a new [`Move`] from the sparse fields.
    #[inline]
    #[must_use]
    pub const fn new(piece: Piece, x: i32, y: i32, rotation: Rotation, spin: Spin) -> Self {
        debug_assert!(x >= 0 && x < WIDTH && y >= 0);
        Self(
            piece as u32
                | ((x as u32) << X_SHIFT)
                | ((y as u32) << Y_SHIFT)
                | ((rotation as u32) << ROT_SHIFT)
                | ((spin as u32) << SPIN_SHIFT),
        )
    }

    /// Decodes the [`Piece`] component.
    ///
    /// # Panics
    /// Panics if the piece is out of range (0..7). This should never happen if the
    /// [`Move`] was created with [`Move::new`].
    #[inline]
    #[must_use]
    pub const fn piece(self) -> Piece {
        Piece::from_u8(((self.0 >> PIECE_SHIFT) & PIECE_MASK) as u8).unwrap()
    }

    /// Decodes the x-position component.
    #[inline]
    #[must_use]
    pub const fn x(self) -> i32 {
        ((self.0 >> X_SHIFT) & X_MASK) as i32
    }

    /// Decodes the y-position component.
    #[inline]
    #[must_use]
    pub const fn y(self) -> i32 {
        ((self.0 >> Y_SHIFT) & Y_MASK) as i32
    }

    /// Decodes the [`Rotation`] component.
    #[inline]
    #[must_use]
    pub const fn rotation(self) -> Rotation {
        Rotation::from_u8(((self.0 >> ROT_SHIFT) & ROT_MASK) as u8)
    }

    /// Decodes the [`Spin`] component.
    #[inline]
    #[must_use]
    pub const fn spin(self) -> Spin {
        let val = ((self.0 >> SPIN_SHIFT) & SPIN_MASK) as u8;
        match val {
            1 => Spin::Mini,
            2 => Spin::Full,
            // The value 3 is invalid. Discard it as a non-spin.
            _ => Spin::None,
        }
    }

    /// Returns the four absolute cells this placement would occupy.
    /// These cells are not guaranteed to be within `(0..4, 0..64)`.
    #[inline]
    #[must_use]
    pub const fn cells(self) -> [(i32, i32); 4] {
        let p = self.piece();
        let r = self.rotation() as usize;
        let x = self.x();
        let y = self.y();
        let base = p.base_cells();
        let rotated = [
            rot_cell(base[0], r),
            rot_cell(base[1], r),
            rot_cell(base[2], r),
        ];
        [
            (x, y),
            (x + rotated[0].0 as i32, y + rotated[0].1 as i32),
            (x + rotated[1].0 as i32, y + rotated[1].1 as i32),
            (x + rotated[2].0 as i32, y + rotated[2].1 as i32),
        ]
    }

    /// Returns the canonical form of this [`Move`].
    ///
    /// Symmetrical [`Move`]s can have the same [`Move::cells`] result, with different
    /// values:
    /// - [`Piece::T`], [`Piece::J`], and [`Piece::L`] has 1 canonical state
    /// - [`Piece::I`], [`Piece::S`], and [`Piece::Z`] have 2 canonical states
    /// - [`Piece::O`] has 1 canonical state
    #[inline]
    #[must_use]
    pub const fn canonicalize(self) -> Self {
        let p = self.piece();
        let r = self.rotation();
        let cr = p.canonical_rotation(r);

        if p.group4() || (r as u8 == cr as u8) {
            self
        } else {
            let (dx, dy) = p.canonical_offset(r);
            Self::new(
                p,
                self.x().saturating_sub(dx),
                self.y().saturating_sub(dy),
                cr,
                self.spin(),
            )
        }
    }
}

impl Debug for Move {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Move")
            .field(&self.piece())
            .field(&self.x())
            .field(&self.y())
            .field(&self.rotation())
            .field(&self.spin())
            .finish()
    }
}

impl Display for Move {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}:{}{}{}",
            self.piece(),
            self.x(),
            self.y(),
            match self.rotation() {
                Rotation::North => "N",
                Rotation::East => "E",
                Rotation::South => "S",
                Rotation::West => "W",
            },
            match self.spin() {
                Spin::None => "n",
                Spin::Mini => "m",
                Spin::Full => "f",
            }
        )
    }
}
