//! Compact move packing utilities and cell reconstruction helpers.

use std::fmt::Debug;

use crate::header::rot_cell;
use crate::piece::Piece;
use crate::rotation::Rotation;
use crate::spin::Spin;

/// A move is a 32-bit integer with the following layout:
/// Piece (3 bits) | Rotation (2 bits) | X (4 bits) | Y (8 bits) | Spin (2 bits) | Unused (13 bits)
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move(u32);

impl Move {
    #[must_use]
    /// Packs move fields into a compact 32-bit representation.
    pub const fn new(piece: Piece, rotation: Rotation, x: i32, y: i32, spin: Spin) -> Self {
        Self(
            ((piece as u32) << 29)
                | ((rotation as u32) << 27)
                | (((x as u32) & 0xF) << 23)
                | (((y as u32) & 0xFF) << 15)
                | ((spin as u32) << 13),
        )
    }

    #[must_use]
    /// Decodes the piece component.
    pub const fn piece(&self) -> Piece {
        Piece::from_u8((self.0 >> 29) as u8).unwrap()
    }

    #[must_use]
    /// Decodes the rotation component.
    pub const fn rotation(&self) -> Rotation {
        match (self.0 >> 27) & 0x3 {
            0 => Rotation::North,
            1 => Rotation::East,
            2 => Rotation::South,
            _ => Rotation::West,
        }
    }

    #[must_use]
    /// Decodes the x-coordinate component.
    pub const fn x(&self) -> i32 {
        ((self.0 >> 23) & 0xF) as i32
    }

    #[must_use]
    /// Decodes the y-coordinate component.
    pub const fn y(&self) -> i32 {
        ((self.0 >> 15) & 0xFF) as i32
    }

    #[must_use]
    /// Decodes the spin classification component.
    pub const fn spin(&self) -> Spin {
        match (self.0 >> 13) & 0x3 {
            0 => Spin::None,
            1 => Spin::Mini,
            2 => Spin::Full,
            _ => unreachable!(),
        }
    }

    #[must_use]
    /// Expands the placement into four absolute board cells.
    pub const fn cells(&self) -> [(i32, i32); 4] {
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
}

impl Debug for Move {
    /// Formats the move as a tuple of decoded fields.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Move")
            .field(&self.piece())
            .field(&self.rotation())
            .field(&self.x())
            .field(&self.y())
            .field(&self.spin())
            .finish()
    }
}
