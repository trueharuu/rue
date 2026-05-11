use crate::{board::is_ok_x, coordinates::Coordinates, piece::Piece, placement::piece_table, rotation::{ROTATION_NB, Rotation}};

pub const SPAWN_COL: usize = 4;
pub fn in_bounds(p: Piece, r: Rotation, x: i32) -> bool {
    if !is_ok_x(x) {
        return false;
    }
    let pc = piece_table(p, r);
    is_ok_x(pc[0].x as i32 + x) && is_ok_x(pc[1].x as i32 + x) && is_ok_x(pc[2].x as i32 + x)
}

pub const fn group2(p: Piece) -> bool {
    matches!(p, Piece::I | Piece::S | Piece::Z)
}

pub const fn canonical_size(p: Piece) -> usize {
    match p {
        Piece::O => 1,
        Piece::I | Piece::S | Piece::Z => 2,
        _ => 4, // L, J, T
    }
}

pub fn canonical_r(p: Piece, r: Rotation) -> Rotation {
    match p {
        Piece::O => Rotation::North,
        Piece::I | Piece::S | Piece::Z => {
            Rotation::from_u8((r as u8) & 1)
        }
        Piece::T | Piece::J | Piece::L => r, 
    }
}

pub fn canonical_offset(p: Piece, r: Rotation) -> Coordinates {
    match p {
        Piece::I => match r {
            Rotation::South => Coordinates::new(1, 0),
            Rotation::West => Coordinates::new(0, -1),
            _ => Coordinates::new(0, 0),
        },
        Piece::S | Piece::Z => match r {
            Rotation::South => Coordinates::new(0, 1),
            Rotation::West => Coordinates::new(1, 0),
            _ => Coordinates::new(0, 0),
        },
        _ => Coordinates::new(0, 0),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Direction {
    Cw = 0,
    Ccw = 1,
    Flip = 2,
}

pub const DIRECTION_NB: usize = 2;

pub fn rotate(d: Direction, r: Rotation) -> Rotation {
    let ri = r as u8;
    let result = match d {
        Direction::Cw => (ri + 1) & 3,
        Direction::Ccw => (ri + 3) & 3,
        Direction::Flip => (ri + 2) & 3,
    };
    Rotation::from_u8(result)
}

pub type Offsets5 = [Coordinates; 5];
pub type Offsets6 = [Coordinates; 6];

pub macro c($x:expr, $y:expr) {
    Coordinates { x: $x, y: $y }
}

pub static KICKS: [[[Offsets5; ROTATION_NB]; DIRECTION_NB]; 3] = [
    // [0] LJSZT
    [
        // Cw
        [
            [c!(0, 0), c!(-1, 0), c!(-1, 1), c!(0, -2), c!(-1, -2)],
            [c!(0, 0), c!(1, 0), c!(1, -1), c!(0, 2), c!(1, 2)],
            [c!(0, 0), c!(1, 0), c!(1, 1), c!(0, -2), c!(1, -2)],
            [c!(0, 0), c!(-1, 0), c!(-1, -1), c!(0, 2), c!(-1, 2)],
        ],
        // CCW
        [
            [c!(0, 0), c!(1, 0), c!(1, 1), c!(0, -2), c!(1, -2)],
            [c!(0, 0), c!(1, 0), c!(1, -1), c!(0, 2), c!(1, 2)],
            [c!(0, 0), c!(-1, 0), c!(-1, 1), c!(0, -2), c!(-1, -2)],
            [c!(0, 0), c!(-1, 0), c!(-1, -1), c!(0, 2), c!(-1, 2)],
        ],
    ],
    // [1] I SRS
    [
        // CW
        [
            [c!(1, 0), c!(-1, 0), c!(2, 0), c!(-1, -1), c!(2, 2)],
            [c!(0, -1), c!(-1, -1), c!(2, -1), c!(-1, 1), c!(2, -2)],
            [c!(-1, 0), c!(1, 0), c!(-2, 0), c!(1, 1), c!(-2, -2)],
            [c!(0, 1), c!(1, 1), c!(-2, 1), c!(1, -1), c!(-2, 2)],
        ],
        // CCW
        [
            [c!(0, -1), c!(-1, -1), c!(2, -1), c!(-1, 1), c!(2, -2)],
            [c!(-1, 0), c!(1, 0), c!(-2, 0), c!(1, 1), c!(-2, -2)],
            [c!(0, 1), c!(1, 1), c!(-2, 1), c!(1, -1), c!(-2, 2)],
            [c!(1, 0), c!(-1, 0), c!(2, 0), c!(-1, -1), c!(2, 2)],
        ],
    ],
    // [2] I SRS+
    [
        // CW
        [
            [c!(1, 0), c!(2, 0), c!(-1, 0), c!(-1, -1), c!(2, 2)],
            [c!(0, -1), c!(-1, -1), c!(2, -1), c!(-1, 1), c!(2, -2)],
            [c!(-1, 0), c!(1, 0), c!(-2, 0), c!(1, 1), c!(-2, -2)],
            [c!(0, 1), c!(1, 1), c!(-2, 1), c!(1, -1), c!(-2, 2)],
        ],
        // CCW
        [
            [c!(0, -1), c!(-1, -1), c!(2, -1), c!(2, -2), c!(-1, 1)],
            [c!(-1, 0), c!(-2, 0), c!(1, 0), c!(-2, -2), c!(1, 1)],
            [c!(0, 1), c!(-2, 1), c!(1, 1), c!(-2, 2), c!(1, -1)],
            [c!(1, 0), c!(2, 0), c!(-1, 0), c!(2, 2), c!(-1, -1)],
        ],
    ],
];

pub static KICKS_180: [[Offsets6; ROTATION_NB]; 2] = [
    // [0] LJSZT
    [
        [c!(0, 0), c!(0, 1), c!(1, 1), c!(-1, 1), c!(1, 0), c!(-1, 0)],
        [c!(0, 0), c!(1, 0), c!(1, 2), c!(1, 1), c!(0, 2), c!(0, 1)],
        [
            c!(0, 0),
            c!(0, -1),
            c!(-1, -1),
            c!(1, -1),
            c!(-1, 0),
            c!(1, 0),
        ],
        [
            c!(0, 0),
            c!(-1, 0),
            c!(-1, 2),
            c!(-1, 1),
            c!(0, 2),
            c!(0, 1),
        ],
    ],
    // [1] I
    [
        [
            c!(1, -1),
            c!(1, 0),
            c!(2, 0),
            c!(0, 0),
            c!(2, -1),
            c!(0, -1),
        ],
        [
            c!(-1, -1),
            c!(0, -1),
            c!(0, 1),
            c!(0, 0),
            c!(-1, 1),
            c!(-1, 0),
        ],
        [
            c!(-1, 1),
            c!(-1, 0),
            c!(-2, 0),
            c!(0, 0),
            c!(-2, 1),
            c!(0, 1),
        ],
        [c!(1, 1), c!(0, 1), c!(0, 3), c!(0, 2), c!(1, 3), c!(1, 2)],
    ],
];

pub fn kick_index(p: Piece, srs_plus: bool) -> usize {
    let is_i = (p == Piece::I) as usize;
    if srs_plus {
        is_i * 2
    } else {
        is_i
    }
}

pub fn kick_180_index(p: Piece) -> usize {
    (p == Piece::I) as usize
}