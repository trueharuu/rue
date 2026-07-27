//! Rotation states and orientation transforms.

use std::marker::ConstParamTy;

/// Cardinal orientation for piece geometry.
#[derive(Debug, Clone, Copy, ConstParamTy)]
#[derive_const(PartialEq, Eq)]
pub enum Rotation {
    /// Spawn orientation.
    North = 0,
    /// 90-degree clockwise from spawn.
    East = 1,
    /// 180-degree rotation from spawn.
    South = 2,
    /// 90-degree counterclockwise from spawn.
    West = 3,
}

impl Rotation {
    /// Number of rotation states.
    pub const NB: usize = 4;
    /// All rotation states in index order.
    pub const ALL: [Rotation; Self::NB] = [
        Rotation::North,
        Rotation::East,
        Rotation::South,
        Rotation::West,
    ];

    #[must_use]
    /// Converts an integer to a rotation by masking to the lowest two bits.
    pub const fn from(r: u8) -> Self {
        match r & 3 {
            0 => Rotation::North,
            1 => Rotation::East,
            2 => Rotation::South,
            3 => Rotation::West,
            _ => unreachable!(),
        }
    }

    #[must_use]
    /// Returns the clockwise successor rotation.
    pub const fn cw(self) -> Self {
        match self {
            Rotation::North => Rotation::East,
            Rotation::East => Rotation::South,
            Rotation::South => Rotation::West,
            Rotation::West => Rotation::North,
        }
    }

    #[must_use]
    /// Returns the counterclockwise successor rotation.
    pub const fn ccw(self) -> Self {
        match self {
            Rotation::North => Rotation::West,
            Rotation::East => Rotation::North,
            Rotation::South => Rotation::East,
            Rotation::West => Rotation::South,
        }
    }

    #[must_use]
    /// Returns the half-turn successor rotation.
    pub const fn flip(self) -> Self {
        match self {
            Rotation::North => Rotation::South,
            Rotation::East => Rotation::West,
            Rotation::South => Rotation::North,
            Rotation::West => Rotation::East,
        }
    }
}
