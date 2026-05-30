use std::fmt;
use std::rc::Rc;

use rue_core::board::{Bitboard, Board, COL_NB};
use rue_core::data::{
    Direction, KICKS, KICKS_180, SPAWN_COL, canonical_offset, canonical_r, kick_180_index,
    kick_index,
};
use rue_core::piece::Piece;
use rue_core::placement::Move;
use rue_core::rotation::{ALL_ROTATIONS, ROTATION_NB, Rotation};
use rue_core::ruleset::ACTIVE_RULES;
use rue_core::spin::{SPIN_NB, SpinType};

use crate::collision_map::CollisionMap;
use crate::movegen::{bb, bb_low};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MovementAction {
    Spawn,
    TapLeft,
    TapRight,
    DASLeft,
    DASRight,
    Softdrop,
    Hold,
    RotateCW,
    RotateCCW,
    Rotate180,
    Harddrop,
}

impl fmt::Display for MovementAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MovementAction::Spawn => "spawn",
            MovementAction::TapLeft => "moveLeft",
            MovementAction::TapRight => "moveRight",
            MovementAction::DASLeft => "dasLeft",
            MovementAction::DASRight => "dasRight",
            MovementAction::Softdrop => "softDrop",
            MovementAction::Hold => "hold",
            MovementAction::RotateCW => "rotateCW",
            MovementAction::RotateCCW => "rotateCCW",
            MovementAction::Rotate180 => "rotate180",
            MovementAction::Harddrop => "hardDrop",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone)]
pub struct InputSequence {
    pub data: Vec<MovementAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Loc {
    piece: Piece,
    x: i32,
    y: i32,
    rotation: Rotation,
    spin: SpinType,
}

#[derive(Debug, Clone)]
struct Node {
    parent: Option<Rc<Node>>,
    action: MovementAction,
    loc: Loc,
}

fn cm_at(cm: &CollisionMap, piece: Piece, x: i32, r: Rotation) -> Bitboard {
    if x < 0 || x >= COL_NB as i32 {
        return !0u64;
    }
    cm.get(x as usize, canonical_r(piece, r))
}

fn obstructed(cm: &CollisionMap, loc: &Loc) -> bool {
    if loc.y < 0 {
        return true;
    }
    let cm_col = cm_at(cm, loc.piece, loc.x, loc.rotation);
    cm_col & bb(loc.y) != 0
}

impl MovementAction {
    fn move_left(cm: &CollisionMap, loc: &Loc, n: i32) -> Loc {
        let mut cur = *loc;
        for _ in 0..n {
            let next = Loc {
                x: cur.x - 1,
                y: cur.y,
                rotation: cur.rotation,
                piece: cur.piece,
                spin: SpinType::None,
            };
            if obstructed(cm, &next) {
                break;
            }
            cur = next;
        }
        cur
    }

    fn move_right(cm: &CollisionMap, loc: &Loc, n: i32) -> Loc {
        let mut cur = *loc;
        for _ in 0..n {
            let next = Loc {
                x: cur.x + 1,
                y: cur.y,
                rotation: cur.rotation,
                piece: cur.piece,
                spin: SpinType::None,
            };
            if obstructed(cm, &next) {
                break;
            }
            cur = next;
        }
        cur
    }

    fn drop_down(cm: &CollisionMap, loc: &Loc) -> Loc {
        let cm_col = cm_at(cm, loc.piece, loc.x, loc.rotation);
        let mask = cm_col & bb_low(loc.y + 1);
        let new_y = 64 - mask.leading_zeros() as i32;
        Loc {
            x: loc.x,
            y: new_y,
            piece: loc.piece,
            rotation: loc.rotation,
            spin: if loc.y == new_y {
                loc.spin
            } else {
                SpinType::None
            },
        }
    }

    fn rotate(
        cm: &CollisionMap,
        fullspinmap: &[[Bitboard; COL_NB]; ROTATION_NB],
        spinmap: &[[Bitboard; COL_NB]; ROTATION_NB],
        immobile_spinmap: &[[Bitboard; COL_NB]; ROTATION_NB],
        loc: &Loc,
        to: Rotation,
    ) -> Loc {
        if loc.piece == Piece::O {
            return *loc;
        }
        let from = loc.rotation;
        let p = loc.piece;
        let ki = kick_index(p, ACTIVE_RULES.srs_plus);
        let dir = if to == from.rotate_cw() {
            Direction::Cw
        } else if to == from.rotate_ccw() {
            Direction::Ccw
        } else {
            return *loc;
        };

        let off = canonical_offset(p, from) - canonical_offset(p, to);
        let kicks = &KICKS[ki][dir as usize][from as usize];
        let n = if !ACTIVE_RULES.srs_plus && kicks.len() == 6 {
            2
        } else {
            kicks.len()
        };

        for (i, kick) in kicks.iter().enumerate().take(n) {
            let x = loc.x + kick.x as i32 + off.x as i32;
            let y = loc.y + kick.y as i32 + off.y as i32;
            if x < 0 || x >= COL_NB as i32 || y < 0 {
                continue;
            }
            let next = Loc {
                x,
                y,
                piece: loc.piece,
                rotation: to,
                spin: SpinType::None,
            };
            if obstructed(cm, &next) {
                continue;
            }

            let to_canon = canonical_r(p, to);
            let spin = if p == Piece::T {
                if i >= 4 || (fullspinmap[to_canon as usize][x as usize] & bb(y) != 0) {
                    SpinType::Full
                } else if spinmap[to_canon as usize][x as usize] & bb(y) != 0 {
                    SpinType::Mini
                } else {
                    SpinType::None
                }
            } else if immobile_spinmap[to_canon as usize][x as usize] & bb(y) != 0 {
                SpinType::Mini
            } else {
                SpinType::None
            };

            return Loc {
                x,
                y,
                piece: loc.piece,
                rotation: to,
                spin,
            };
        }

        *loc
    }

    fn rotate_180(
        cm: &CollisionMap,
        fullspinmap: &[[Bitboard; COL_NB]; ROTATION_NB],
        spinmap: &[[Bitboard; COL_NB]; ROTATION_NB],
        immobile_spinmap: &[[Bitboard; COL_NB]; ROTATION_NB],
        loc: &Loc,
    ) -> Loc {
        if !ACTIVE_RULES.enable_180 {
            return *loc;
        }
        if loc.piece == Piece::O {
            return *loc;
        }

        let p = loc.piece;
        let from = loc.rotation;
        let to = from.rotate_180();
        let off = canonical_offset(p, from) - canonical_offset(p, to);
        let ki = kick_180_index(p);
        let kicks = &KICKS_180[ki][from as usize];
        let n = if !ACTIVE_RULES.srs_plus && kicks.len() == 6 {
            2
        } else {
            kicks.len()
        };

        for kick in kicks.iter().take(n) {
            let x = loc.x + kick.x as i32 + off.x as i32;
            let y = loc.y + kick.y as i32 + off.y as i32;
            if x < 0 || x >= COL_NB as i32 || y < 0 {
                continue;
            }
            let next = Loc {
                x,
                y,
                piece: loc.piece,
                rotation: to,
                spin: SpinType::None,
            };
            if obstructed(cm, &next) {
                continue;
            }

            let to_canon = canonical_r(p, to);
            let spin = if p == Piece::T {
                if fullspinmap[to_canon as usize][x as usize] & bb(y) != 0 {
                    SpinType::Full
                } else if spinmap[to_canon as usize][x as usize] & bb(y) != 0 {
                    SpinType::Mini
                } else {
                    SpinType::None
                }
            } else if immobile_spinmap[to_canon as usize][x as usize] & bb(y) != 0 {
                SpinType::Mini
            } else {
                SpinType::None
            };

            return Loc {
                x,
                y,
                piece: loc.piece,
                rotation: to,
                spin,
            };
        }

        *loc
    }
}

fn matches_target(loc: &Loc, target: &Move) -> bool {
    if loc.piece != target.piece() {
        return false;
    }
    if loc.x != target.x() || loc.y != target.y() {
        return false;
    }
    let target_rot = canonical_r(loc.piece, target.rotation());
    if canonical_r(loc.piece, loc.rotation) != target_rot {
        return false;
    }
    loc.spin == target.spin()
}

pub fn keygen(board: &Board, target: &Move, human: bool) -> Option<Vec<MovementAction>> {
    let piece = target.piece();
    let cols = board.compute_cols();
    let cm = CollisionMap::new(&cols, piece);

    let mut searched: [[[Bitboard; COL_NB]; ROTATION_NB]; SPIN_NB] =
        [[[0u64; COL_NB]; ROTATION_NB]; SPIN_NB];

    let mut to_search = vec![Node {
        parent: None,
        loc: Loc {
            piece,
            x: SPAWN_COL as i32,
            y: ACTIVE_RULES.spawn_row,
            rotation: Rotation::North,
            spin: SpinType::None,
        },
        action: MovementAction::Spawn,
    }];

    let mut fullspinmap = [[0u64; COL_NB]; ROTATION_NB];
    let mut spinmap = [[0u64; COL_NB]; ROTATION_NB];
    let mut immobile_spinmap = [[0u64; COL_NB]; ROTATION_NB];

    for x in 0..COL_NB {
        let left = if x > 0 { cols[x - 1] } else { !0u64 };
        let right = if x + 1 < COL_NB { cols[x + 1] } else { !0u64 };
        let corners = [left >> 1, right >> 1, (right << 1) | 1, (left << 1) | 1];

        let spins = (corners[0] & corners[1] & (corners[2] | corners[3]))
            | (corners[2] & corners[3] & (corners[0] | corners[1]));

        for r in ALL_ROTATIONS {
            let rc = canonical_r(piece, r);
            if piece == Piece::T {
                spinmap[rc as usize][x] = spins;
            }

            let cm_col = cm_at(&cm, piece, x as i32, r);
            if cm_col != !0u64 {
                if piece == Piece::T {
                    let cw = r.rotate_cw();
                    fullspinmap[rc as usize][x] =
                        spins & corners[r as usize] & corners[cw as usize];
                }

                let left_cm = if x > 0 {
                    cm_at(&cm, piece, (x - 1) as i32, r)
                } else {
                    !0u64
                };
                let right_cm = if x + 1 < COL_NB {
                    cm_at(&cm, piece, (x + 1) as i32, r)
                } else {
                    !0u64
                };

                let stuck = !cm_col & left_cm & right_cm & ((cm_col << 1) | 1) & (cm_col >> 1);
                immobile_spinmap[rc as usize][x] |= stuck;
            }
        }
    }

    let mut found_node: Option<Node> = None;

    while !to_search.is_empty() && found_node.is_none() {
        let mut new_search = Vec::new();
        for node in to_search.into_iter().map(Rc::new) {
            let mut push_loc = |l: Loc, action: MovementAction| {
                if found_node.is_some() {
                    return;
                }
                if obstructed(&cm, &l) {
                    return;
                }

                let new_loc = match action {
                    MovementAction::TapLeft => MovementAction::move_left(&cm, &l, 1),
                    MovementAction::TapRight => MovementAction::move_right(&cm, &l, 1),
                    MovementAction::DASLeft => MovementAction::move_left(&cm, &l, l.x),
                    MovementAction::DASRight => {
                        MovementAction::move_right(&cm, &l, (COL_NB - 1) as i32 - l.x)
                    }
                    MovementAction::Softdrop | MovementAction::Harddrop => {
                        MovementAction::drop_down(&cm, &l)
                    }
                    MovementAction::RotateCW => MovementAction::rotate(
                        &cm,
                        &fullspinmap,
                        &spinmap,
                        &immobile_spinmap,
                        &l,
                        l.rotation.rotate_cw(),
                    ),
                    MovementAction::RotateCCW => MovementAction::rotate(
                        &cm,
                        &fullspinmap,
                        &spinmap,
                        &immobile_spinmap,
                        &l,
                        l.rotation.rotate_ccw(),
                    ),
                    MovementAction::Rotate180 => MovementAction::rotate_180(
                        &cm,
                        &fullspinmap,
                        &spinmap,
                        &immobile_spinmap,
                        &l,
                    ),
                    _ => l,
                };

                let new_node = Node {
                    loc: new_loc,
                    parent: Some(Rc::clone(&node)),
                    action,
                };

                if action == MovementAction::Harddrop {
                    if matches_target(&new_node.loc, target) {
                        found_node = Some(new_node);
                    }
                    return;
                }

                if new_loc.x < 0 || new_loc.x >= COL_NB as i32 || new_loc.y < 0 {
                    return;
                }

                let searched_col = &mut searched[new_loc.spin as usize][new_loc.rotation as usize]
                    [new_loc.x as usize];
                if *searched_col & bb(new_loc.y) == 0 {
                    *searched_col |= bb(new_loc.y);
                    new_search.push(new_node);
                }
            };

            push_loc(node.loc, MovementAction::Harddrop);
            if human {
                push_loc(node.loc, MovementAction::DASLeft);
                push_loc(node.loc, MovementAction::DASRight);
            }
            push_loc(node.loc, MovementAction::TapLeft);
            push_loc(node.loc, MovementAction::TapRight);
            push_loc(node.loc, MovementAction::Softdrop);
            push_loc(node.loc, MovementAction::RotateCW);
            push_loc(node.loc, MovementAction::RotateCCW);
            push_loc(node.loc, MovementAction::Rotate180);
        }
        to_search = new_search;
    }

    let mut found_node = match found_node {
        Some(node) => Rc::new(node),
        None => return None,
    };

    let mut moves = Vec::new();
    while found_node.parent.is_some() {
        moves.push(found_node.action);
        found_node = found_node.parent.as_ref().unwrap().clone();
    }

    moves.reverse();
    Some(moves)
}

pub fn get_input(board: &Board, target: &Move, hold_used: bool, human: bool) -> InputSequence {
    let mut data = keygen(board, target, human)
        .unwrap_or_else(|| vec![MovementAction::Harddrop, MovementAction::Harddrop]);
    if hold_used {
        data.insert(0, MovementAction::Hold);
    }
    InputSequence { data }
}
