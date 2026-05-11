use crate::coordinates::Coordinates;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Rotation {
    North = 0,
    East = 1,
    South = 2,
    West = 3,
}

impl Rotation {
    pub const fn from_u8(v: u8) -> Self {
        match v {
            0 => Rotation::North,
            1 => Rotation::East,
            2 => Rotation::South,
            3 => Rotation::West,
            _ => panic!("invalid Rotation discriminant"),
        }
    }

    pub const fn rotate_cw(self) -> Self {
        Self::from_u8((self as u8 + 1) % ROTATION_NB as u8)
    }

    pub const fn rotate_ccw(self) -> Self {
        Self::from_u8((self as u8 + ROTATION_NB as u8 - 1) % ROTATION_NB as u8)
    }

    pub const fn rotate_180(self) -> Self {
        Self::from_u8((self as u8 + 2) % ROTATION_NB as u8)
    }
}

pub const ROTATION_NB: usize = 4;

pub const ALL_ROTATIONS: [Rotation; ROTATION_NB] = [
    Rotation::North,
    Rotation::East,
    Rotation::South,
    Rotation::West,
];

pub const fn is_ok_rotation(r: Rotation) -> bool {
    (r as u8) < ROTATION_NB as u8
}

pub const fn rotate_coord(r: Rotation, c: Coordinates) -> Coordinates {
    match r {
        Rotation::East => Coordinates::new(c.y as i32, -(c.x as i32)),
        Rotation::South => Coordinates::new(-(c.x as i32), -(c.y as i32)),
        Rotation::West => Coordinates::new(-(c.y as i32), c.x as i32),
        Rotation::North => c,
    }
}