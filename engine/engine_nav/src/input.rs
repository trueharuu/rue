use std::fmt::Display;

use engine_core::{
    board::Board, piece::Mino, piece_location::PieceLocation, rotation::Rotation, spin::Spin,
};
use serde::Serialize;

use crate::{
    collision_map::CollisionMap,
    movegen::{bb, bb_low, kicks, kicks_180},
};

#[derive(Debug, Copy, Clone, Serialize, PartialEq, Eq)]
pub enum Move {
    Spawn,
    #[serde(rename = "moveLeft")]
    TapLeft,
    #[serde(rename = "moveRight")]
    TapRight,
    // #[serde(rename = "dasLeft")]
    // DasLeft,
    // #[serde(rename = "dasRight")]
    // DasRight,
    #[serde(rename = "softDrop")]
    SoftDrop,
    #[serde(rename = "hold")]
    Hold,
    #[serde(rename = "rotateCCW")]
    RotateCCW,
    #[serde(rename = "rotateCW")]
    RotateCW,
    #[serde(rename = "rotate180")]
    Rotate180,
    #[serde(rename = "hardDrop")]
    HardDrop,
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Move::Spawn => write!(f, "spawn"),
            Move::TapLeft => write!(f, "moveLeft"),
            Move::TapRight => write!(f, "moveRight"),
            // Move::DasLeft => write!(f, "dasLeft"),
            // Move::DasRight => write!(f, "dasRight"),
            Move::SoftDrop => write!(f, "softDrop"),
            Move::Hold => write!(f, "hold"),
            Move::RotateCCW => write!(f, "rotateCCW"),
            Move::RotateCW => write!(f, "rotateCW"),
            Move::Rotate180 => write!(f, "rotate180"),
            Move::HardDrop => write!(f, "hardDrop"),
        }
    }
}

impl Move {
    pub fn move_left(cm: &[CollisionMap; 4], loc: &PieceLocation, n: i8) -> PieceLocation {
        if n == 0 {
            return loc.clone();
        }
        let new_loc = PieceLocation {
            x: loc.x - 1,
            y: loc.y,
            rotation: loc.rotation,
            piece: loc.piece,
            spin: Spin::None,
        };
        if cm[loc.rotation as usize].obstructed(new_loc.x, new_loc.y) {
            loc.clone()
        } else {
            Self::move_left(cm, &new_loc, n - 1)
        }
    }

    pub fn move_right(cm: &[CollisionMap; 4], loc: &PieceLocation, n: i8) -> PieceLocation {
        if n == 0 {
            return loc.clone();
        }
        let new_loc = PieceLocation {
            x: loc.x + 1,
            y: loc.y,
            rotation: loc.rotation,
            piece: loc.piece,
            spin: Spin::None,
        };
        if cm[loc.rotation as usize].obstructed(new_loc.x, new_loc.y) {
            loc.clone()
        } else {
            Self::move_right(cm, &new_loc, n - 1)
        }
    }

    pub fn drop_down(cm: &[CollisionMap; 4], loc: &PieceLocation) -> PieceLocation {
        let new_y = 64
            - (cm[loc.rotation as usize][loc.x as usize] & bb_low(loc.y + 1)).leading_zeros() as i8;
        PieceLocation {
            x: loc.x,
            y: new_y,
            piece: loc.piece,
            rotation: loc.rotation,
            spin: if loc.y == new_y { loc.spin } else { Spin::None },
        }
    }

    pub fn rotate(
        cm: &[CollisionMap; 4],
        fullspinmap: &[Board; 4],
        spinmap: &[Board; 4],
        immobile_spinmap: &[Board; 4],
        loc: &PieceLocation,
        to: Rotation,
    ) -> PieceLocation {
        let cmr = &cm[to as usize];
        let kcks = kicks(loc.piece, loc.rotation, to);
        for i in 0..5 {
            let (kx, ky) = kcks[i];
            let (x, y) = (loc.x + kx, loc.y + ky);
            if !cmr.obstructed(x, y) {
                let spin = if loc.piece == Mino::T {
                    if i >= 4 {
                        Spin::Full
                    } else {
                        if fullspinmap[to as usize][x as usize] & bb(y) > 0 {
                            Spin::Full
                        } else if spinmap[to as usize][x as usize] & bb(y) > 0 {
                            Spin::Mini
                        } else {
                            Spin::None
                        }
                    }
                } else {
                    if immobile_spinmap[to as usize][x as usize] & bb(y) > 0 {
                        Spin::Mini
                    } else {
                        Spin::None
                    }
                };
                return PieceLocation {
                    x,
                    y,
                    piece: loc.piece,
                    rotation: to,
                    spin,
                };
            }
        }
        loc.clone()
    }

    pub fn rotate_180(
        cm: &[CollisionMap; 4],
        fullspinmap: &[Board; 4],
        spinmap: &[Board; 4],
        immobile_spinmap: &[Board; 4],
        loc: &PieceLocation,
    ) -> PieceLocation {
        let to = loc.rotation.rotate_180();
        let cmr = &cm[to as usize];
        let kcks = kicks_180(loc.piece, loc.rotation, to);
        for i in 0..6 {
            let (kx, ky) = kcks[i];
            let (x, y) = (loc.x + kx, loc.y + ky);
            if !cmr.obstructed(x, y) {
                let spin = if loc.piece == Mino::T {
                    if fullspinmap[to as usize][x as usize] & bb(y) > 0 {
                        Spin::Full
                    } else if spinmap[to as usize][x as usize] & bb(y) > 0 {
                        Spin::Mini
                    } else {
                        Spin::None
                    }
                } else {
                    if immobile_spinmap[to as usize][x as usize] & bb(y) > 0 {
                        Spin::Mini
                    } else {
                        Spin::None
                    }
                };
                return PieceLocation {
                    x,
                    y,
                    piece: loc.piece,
                    rotation: to,
                    spin,
                };
            }
        }
        loc.clone()
    }
}
