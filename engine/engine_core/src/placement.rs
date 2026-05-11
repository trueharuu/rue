use std::fmt::Debug;

use crate::{board::{is_ok_x, is_ok_y}, coordinates::Coordinates, piece::{Piece, PieceCoordinates, TSPIN, make_piece}, rotation::{Rotation, rotate_coord}, spin::SpinType};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move {
    data: u16,
}

impl Move {
    pub const fn new(p: Piece, r: Rotation, x: i32, y: i32, fullspin: bool) -> Self {
        let piece_val = if fullspin { TSPIN } else { p as u16 };
        let data = (y as u16 & 0x3F)
            | ((x as u16 & 0xF) << 6)
            | ((piece_val & 0x7) << 10)
            | (((r as u16) & 0x3) << 13)
            | ((fullspin as u16) << 15);
        Self { data }
    }

    /// C++ Move(TSPIN, r, x, y, fullspin) — for T-spin move emission
    pub const fn new_tspin(r: Rotation, x: i32, y: i32, fullspin: bool) -> Self {
        let data = (y as u16 & 0x3F)
            | ((x as u16 & 0xF) << 6)
            | ((TSPIN & 0x7) << 10)
            | (((r as u16) & 0x3) << 13)
            | ((fullspin as u16) << 15);
        Self { data }
    }

    /// Allspin mini: stores actual piece (not TSPIN sentinel) + spin_bit=1.
    /// spin() returns (0 + 1) = 1 = Mini.  piece() returns the real piece.
    pub const fn new_allspin_mini(p: Piece, r: Rotation, x: i32, y: i32) -> Self {
        let data = (y as u16 & 0x3F)
            | ((x as u16 & 0xF) << 6)
            | (((p as u16) & 0x7) << 10)
            | (((r as u16) & 0x3) << 13)
            | (1u16 << 15); // spin_bit = 1
        Self { data }
    }

    pub const fn none() -> Self {
        Self { data: 0 }
    }

    pub const fn piece(self) -> Piece {
        let raw = (self.data >> 10) & 0x7;
        if raw == TSPIN {
            // TSPIN maps to T
            Piece::T
        } else {
            Piece::from_u8(raw as u8)
        }
    }

    pub const fn rotation(self) -> Rotation {
        Rotation::from_u8(((self.data >> 13) & 0x3) as u8)
    }

    pub const fn spin(self) -> SpinType {
        let piece_raw = (self.data >> 10) & 0x7;
        let spin_bit = (self.data >> 15) & 0x1;
        let val = (piece_raw == TSPIN) as u8 + spin_bit as u8;
        SpinType::from_u8(val)
    }

    pub const fn x(self) -> i32 {
        ((self.data >> 6) & 0xF) as i32
    }

    pub const fn y(self) -> i32 {
        (self.data & 0x3F) as i32
    }

    pub const fn raw(self) -> u16 {
        self.data
    }

    pub fn cells(self) -> PieceCoordinates {
        piece_table(self.piece(), self.rotation())
    }

    pub fn blocks(self) -> [Coordinates; 4] {
        let pc = self.cells();
        let off = Coordinates::new(self.x(), self.y());
        [
            off,
            pc.coords[0] + off,
            pc.coords[1] + off,
            pc.coords[2] + off,
        ]
    }
}

pub const fn piece_table(p: Piece, r: Rotation) -> PieceCoordinates {
    let cells = make_piece(p);
    PieceCoordinates::new(
        rotate_coord(r, cells.coords[0]),
        rotate_coord(r, cells.coords[1]),
        rotate_coord(r, cells.coords[2]),
    )
}

pub const fn lut_p(p: Piece, r: Rotation) -> [(i8, i8); 4] {
    let cells = make_piece(p);
    [
        (0, 0),
        rotate_coord(r, cells.coords[0]).pair(),
        rotate_coord(r, cells.coords[1]).pair(),
        rotate_coord(r, cells.coords[2]).pair(),
    ]
}

pub fn is_ok_move(m: &Move) -> bool {
    is_ok_x(m.x()) && is_ok_y(m.y())
}

impl Debug for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Move")
            .field("piece", &self.piece())
            .field("rotation", &self.rotation())
            .field("x", &self.x())
            .field("y", &self.y())
            .field("spin", &self.spin())
            .finish()
    }
}