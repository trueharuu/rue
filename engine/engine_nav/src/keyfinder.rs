use std::rc::Rc;

use engine_core::{
    board::Board,
    piece::Mino,
    piece_location::PieceLocation,
    rotation::{ROT, Rotation},
    spin::Spin,
};

use crate::{
    collision_map::CollisionMap,
    input::Move,
    movegen::{SPAWN_COL, SPAWN_ROW, bb},
};

#[derive(Debug, Clone)]
struct Node {
    parent_node: Option<Rc<Node>>, // ?? Rc
    action: Move,
    loc: PieceLocation,
}

pub fn keygen(board: &Board, loc: &PieceLocation, human: bool) -> Option<Vec<Move>> {
    let cm = ROT.map(|r| CollisionMap::new(board, loc.piece, r));
    let mut searched_nodes: [[Board; 4]; 3] =
        std::array::from_fn(|_| std::array::from_fn(|_| Board::new()));
    let mut to_search: Vec<Node> = vec![Node {
        parent_node: None,
        loc: PieceLocation {
            piece: loc.piece,
            x: SPAWN_COL as i8,
            y: SPAWN_ROW,
            rotation: Rotation::North,
            spin: Spin::None,
        },
        action: Move::Spawn,
    }];

    let mut fullspinmap: [Board; 4] = std::array::from_fn(|_| Board::new());
    let mut spinmap: [Board; 4] = std::array::from_fn(|_| Board::new());
    let mut immobile_spinmap: [Board; 4] = std::array::from_fn(|_| Board::new());

    for x in 0..10 {
        let c = [
            if x > 0 { board[x - 1] >> 1 } else { !0 },
            if x < 9 { board[x + 1] >> 1 } else { !0 },
            if x < 9 { board[x + 1] << 1 | 1 } else { !0 },
            if x > 0 { board[x - 1] << 1 | 1 } else { !0 },
        ];

        let spins = c[0] & c[1] & (c[2] | c[3]) | c[2] & c[3] & (c[0] | c[1]);

        for rot in ROT {
            if loc.piece == Mino::T {
                spinmap[rot as usize][x] = spins;
            }
            if cm[rot as usize][x] != !0 {
                if loc.piece == Mino::T {
                    fullspinmap[rot as usize][x] =
                        spins & c[rot as usize] & c[rot.rotate_cw() as usize];
                }
                immobile_spinmap[rot as usize][x] |= !cm[rot as usize][x]
                    & (cm[rot as usize].cols.get(x - 1).copied().unwrap_or(!0)
                        & cm[rot as usize].cols.get(x + 1).copied().unwrap_or(!0)
                        & (cm[rot as usize][x] << 1 | 1)
                        & cm[rot as usize][x] >> 1);
            }
        }
    }

    let mut found_node: Option<Node> = None;

    while to_search.len() > 0 && found_node.is_none() {
        let mut new_search: Vec<Node> = vec![];
        for node in to_search.into_iter().map(Rc::new) {
            let mut push_loc = |l: &PieceLocation, action: Move| {
                if found_node.is_some() {
                    return;
                }
                if cm[l.rotation as usize].obstructed(l.x, l.y) {
                    return;
                }
                let new_node = Node {
                    loc: match action {
                        Move::TapLeft => Move::move_left(&cm, l, 1),
                        Move::TapRight => Move::move_right(&cm, l, 1),
                        // Move::DasLeft => Move::move_left(&cm, l, l.x),
                        // Move::DasRight => Move::move_right(&cm, l, 9 - l.x),
                        Move::SoftDrop | Move::HardDrop => Move::drop_down(&cm, l),
                        Move::RotateCW => Move::rotate(
                            &cm,
                            &fullspinmap,
                            &spinmap,
                            &immobile_spinmap,
                            l,
                            l.rotation.rotate_cw(),
                        ),
                        Move::RotateCCW => Move::rotate(
                            &cm,
                            &fullspinmap,
                            &spinmap,
                            &immobile_spinmap,
                            l,
                            l.rotation.rotate_ccw(),
                        ),
                        Move::Rotate180 => {
                            Move::rotate_180(&cm, &fullspinmap, &spinmap, &immobile_spinmap, l)
                        }
                        _ => l.clone(),
                    },
                    parent_node: Some(Rc::clone(&node)),
                    action,
                };
                let searched = &mut searched_nodes[new_node.loc.spin as usize]
                    [new_node.loc.rotation as usize][new_node.loc.x as usize];
                if action == Move::HardDrop {
                    if new_node.loc.x == loc.x
                        && new_node.loc.y == loc.y
                        && new_node.loc.rotation == loc.rotation
                        && new_node.loc.spin == loc.spin
                    {
                        found_node = Some(new_node);
                    }
                } else if *searched & bb(new_node.loc.y) == 0 {
                    *searched |= bb(new_node.loc.y);
                    new_search.push(new_node);
                }
            };
            push_loc(&node.loc, Move::HardDrop);
            // if human {
            //     push_loc(&node.loc, Move::DasLeft);
            //     push_loc(&node.loc, Move::DasRight);
            // }
            push_loc(&node.loc, Move::TapLeft);
            push_loc(&node.loc, Move::TapRight);
            push_loc(&node.loc, Move::SoftDrop);
            push_loc(&node.loc, Move::RotateCW);
            push_loc(&node.loc, Move::RotateCCW);
            push_loc(&node.loc, Move::Rotate180);
        }
        to_search = new_search;
    }

    if found_node.is_none() {
        return None;
    }

    let mut found_node = &Rc::new(found_node.unwrap());
    let mut moves: Vec<Move> = vec![];

    while found_node.parent_node.is_some() {
        moves.push(found_node.action.clone());
        found_node = found_node.parent_node.as_ref().unwrap();
    }

    moves.reverse();
    Some(moves)
}
