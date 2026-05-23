use serde::{Deserialize, Serialize};

use crate::coordinates::Coordinates;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Piece {
    I = 0,
    O = 1,
    T = 2,
    L = 3,
    J = 4,
    S = 5,
    Z = 6,
}

impl<'de> Deserialize<'de> for Piece {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = char::deserialize(deserializer)?;
        match v {
            'I' | 'i' => Ok(Piece::I),
            'O' | 'o' => Ok(Piece::O),
            'T' | 't' => Ok(Piece::T),
            'L' | 'l' => Ok(Piece::L),
            'J' | 'j' => Ok(Piece::J),
            'S' | 's' => Ok(Piece::S),
            'Z' | 'z' => Ok(Piece::Z),
            _ => Err(serde::de::Error::custom(format!(
                "invalid Piece value: {}",
                v
            ))),
        }
    }
}

impl Serialize for Piece {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let c = match self {
            Piece::I => 'i',
            Piece::O => 'o',
            Piece::T => 't',
            Piece::L => 'l',
            Piece::J => 'j',
            Piece::S => 's',
            Piece::Z => 'z',
        };
        serializer.serialize_char(c)
    }
}

impl Piece {
    pub const fn from_u8(v: u8) -> Self {
        match v {
            0 => Piece::I,
            1 => Piece::O,
            2 => Piece::T,
            3 => Piece::L,
            4 => Piece::J,
            5 => Piece::S,
            6 => Piece::Z,
            _ => panic!("invalid Piece discriminant"),
        }
    }
}

pub const PIECE_NB: usize = 7;
pub const TSPIN: u16 = 7;
pub const NO_PIECE: u8 = 8;

pub const ALL_PIECES: [Piece; PIECE_NB] = [
    Piece::I,
    Piece::O,
    Piece::T,
    Piece::L,
    Piece::J,
    Piece::S,
    Piece::Z,
];

#[derive(Debug, Clone, Copy)]
pub struct PieceCoordinates {
    pub coords: [Coordinates; 3],
}

impl PieceCoordinates {
    pub const fn new(a: Coordinates, b: Coordinates, c: Coordinates) -> Self {
        Self { coords: [a, b, c] }
    }
}

impl std::ops::Index<usize> for PieceCoordinates {
    type Output = Coordinates;
    fn index(&self, i: usize) -> &Self::Output {
        debug_assert!(i < 3);
        &self.coords[i]
    }
}

pub const fn is_ok_piece(p: Piece) -> bool {
    (p as u8) < PIECE_NB as u8
}

pub const fn make_piece(p: Piece) -> PieceCoordinates {
    use Coordinates as C;
    match p {
        Piece::I => PieceCoordinates::new(C::new(-1, 0), C::new(1, 0), C::new(2, 0)),
        Piece::O => PieceCoordinates::new(C::new(1, 0), C::new(0, 1), C::new(1, 1)),
        Piece::T => PieceCoordinates::new(C::new(-1, 0), C::new(1, 0), C::new(0, 1)),
        Piece::L => PieceCoordinates::new(C::new(-1, 0), C::new(1, 0), C::new(1, 1)),
        Piece::J => PieceCoordinates::new(C::new(-1, 0), C::new(1, 0), C::new(-1, 1)),
        Piece::S => PieceCoordinates::new(C::new(-1, 0), C::new(0, 1), C::new(1, 1)),
        Piece::Z => PieceCoordinates::new(C::new(-1, 1), C::new(0, 1), C::new(1, 0)),
    }
}