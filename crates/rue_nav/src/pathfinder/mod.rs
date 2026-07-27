//! Input path-finding for placements.
#![allow(clippy::many_single_char_names)]

pub mod input;
pub mod spin;
#[cfg(test)]
mod test;

use std::collections::HashSet;
use std::collections::VecDeque;

use rue_core::board::Board;
use rue_core::data::KICKS_I;
use rue_core::data::KICKS_I_180;
use rue_core::data::KICKS_I_TETRIO;
use rue_core::data::KICKS_LJSZT;
use rue_core::data::KICKS_LJSZT_180;
use rue_core::data::KICKS_LJSZT_180_TETRIO;
use rue_core::game::ruleset::Handling;
use rue_core::header::SPAWN_X;
use rue_core::header::WIDTH;
use rue_core::piece::Piece;
use rue_core::placement::Move;
use rue_core::rotation::Rotation;
use rue_core::spin::Spin;

use crate::collision::usable_map;
use crate::pathfinder::input::Finesse;
use crate::pathfinder::input::Input;

/// Finds a sequence of inputs that reaches a target placement from the spawn position.
///
/// [`finesse`] determines whether to use DAS or not.
///
/// Returns an empty sequence if the target is unreachable.
#[must_use]
#[inline]
pub fn get_input<const N: usize, const RULE: Handling>(board: &Board<N>, target: Move) -> Finesse {
    match target.piece() {
        Piece::T => get_input_impl::<{ Piece::T }, { RULE }, N>(board, target),
        Piece::I => get_input_impl::<{ Piece::I }, { RULE }, N>(board, target),
        Piece::O => get_input_impl::<{ Piece::O }, { RULE }, N>(board, target),
        Piece::L => get_input_impl::<{ Piece::L }, { RULE }, N>(board, target),
        Piece::J => get_input_impl::<{ Piece::J }, { RULE }, N>(board, target),
        Piece::S => get_input_impl::<{ Piece::S }, { RULE }, N>(board, target),
        Piece::Z => get_input_impl::<{ Piece::Z }, { RULE }, N>(board, target),
    }
}

/// Internal specialized implementation over [`Piece`] for [`get_input`].
#[must_use]
#[inline]
pub fn get_input_impl<const P: Piece, const RULE: Handling, const N: usize>(
    board: &Board<N>,
    target: Move,
) -> Finesse {
    let usable = usable_map::<P, N>(board);

    let mut queue = VecDeque::new();
    let highest = board.max_y() + 2;
    queue.push_back((
        Finesse::new(),
        Move::new(P, Rotation::North, SPAWN_X, highest, Spin::None),
    ));

    let mut seen: HashSet<Move> = HashSet::new();

    while let Some((path, ghost)) = queue.pop_front() {
        if !seen.insert(ghost) {
            // println!("seen {ghost:?}");
            continue;
        }

        let x = ghost.x();
        let y = ghost.y();
        let r = ghost.rotation() as usize;

        // harddrop
        let mut drop_y = y;
        // println!(
        //     "{path:?} {ghost:?} -> {:?} vs {target:?}",
        //     ghost.canonicalize()
        // );
        // println!("{}", render::render_with(*board, &ghost));
        while check::<P, N>(&usable, x, drop_y - 1, r) {
            drop_y -= 1;
        }

        {
            let dropped_ghost = Move::new(
                ghost.piece(),
                ghost.rotation(),
                x,
                y,
                if drop_y == y {
                    ghost.spin()
                } else {
                    Spin::None
                },
            );

            if dropped_ghost.canonicalize() == target.canonicalize() {
                return path.append(Input::HardDrop);
            }
        }

        // extended lateral movement (DAS)
        if RULE.finesse {
            'd: for dx in [-1, 1] {
                let mut x1 = x;
                loop {
                    // dbg!(x1);
                    if !check::<P, N>(&usable, x1, y, r) {
                        break;
                    }

                    x1 += dx;
                }

                x1 -= dx;

                if x1 == x {
                    continue 'd;
                }

                let input = if dx < 0 {
                    Input::DasLeft
                } else {
                    Input::DasRight
                };

                let new_ghost = Move::new(ghost.piece(), ghost.rotation(), x1, y, Spin::None);
                queue.push_back((path.append(input), new_ghost));
            }
        }

        // lateral movement
        'd: for dx in [-1, 1] {
            let x1 = x + dx;
            if !check::<P, N>(&usable, x1, y, r) {
                continue 'd;
            }

            let input = if dx < 0 {
                Input::ShiftLeft
            } else {
                Input::ShiftRight
            };

            let new_ghost = Move::new(ghost.piece(), ghost.rotation(), x1, y, Spin::None);
            queue.push_back((path.append(input), new_ghost));
        }

        // soft drop
        {
            let mut dy = y;
            if RULE.inf_sdf {
                while check::<P, N>(&usable, x, dy - 1, r) {
                    dy -= 1;
                }
            } else if check::<P, N>(&usable, x, dy - 1, r) {
                dy -= 1;
            }

            if dy != y {
                let new_ghost = Move::new(ghost.piece(), ghost.rotation(), x, dy, Spin::None);
                queue.push_back((path.append(Input::SoftDrop), new_ghost));
            }
        }

        // rotation (cw, ccw)
        for dir in [0, 1] {
            let kt = if P == Piece::I {
                if RULE.srs_plus {
                    KICKS_I_TETRIO
                } else {
                    KICKS_I
                }
            } else {
                KICKS_LJSZT
            };

            // try all kicks until one works
            'k: for (i, &(dx, dy)) in kt[dir][r].iter().enumerate() {
                let r1 = if dir == 0 { (r + 1) % 4 } else { (r + 3) % 4 };
                let x1 = x + i32::from(dx);
                let y1 = y + i32::from(dy);

                if !check::<P, N>(&usable, x1, y1, r1) {
                    continue 'k;
                }

                let new_ghost = Move::new(
                    ghost.piece(),
                    Rotation::from(r1 as u8),
                    x1,
                    y1,
                    spin::classify::<P, RULE, N>(board, &usable, x1, y1, r1, i),
                );
                let input = if dir == 0 {
                    Input::RotateCW
                } else {
                    Input::RotateCCW
                };

                queue.push_back((path.append(input), new_ghost));
                break 'k;
            }
        }

        // rotation (180)
        if RULE.use_180 {
            let kt = if P == Piece::I {
                KICKS_I_180
            } else if RULE.srs_plus {
                KICKS_LJSZT_180_TETRIO
            } else {
                KICKS_LJSZT_180
            };

            // try all kicks until one works
            'k: for (dx, dy) in kt[r] {
                let r1 = (r + 2) % 4;
                let x1 = x + i32::from(dx);
                let y1 = y + i32::from(dy);

                if !check::<P, N>(&usable, x1, y1, r1) {
                    continue 'k;
                }

                let new_ghost = Move::new(
                    ghost.piece(),
                    Rotation::from(r1 as u8),
                    x1,
                    y1,
                    // is `0` ok here? is this even possible?
                    spin::classify::<P, RULE, N>(board, &usable, x1, y1, r1, 0),
                );
                queue.push_back((path.append(Input::RotateFlip), new_ghost));
                break 'k;
            }
        }
    }

    Finesse::new()
}

/// Whether a placement is valid, automatically adapting to canonical rotation and offset.
#[must_use]
#[inline]
pub fn check<const P: Piece, const N: usize>(
    usable: &[Board<N>; 4],
    x: i32,
    y: i32,
    r: usize,
) -> bool {
    let (dx, dy) = P.canonical_offset(r);
    let rc = P.canonical_rotation(r);
    (0..WIDTH).contains(&(x - dx))
        && (0..Board::<N>::H).contains(&(y - dy))
        && usable[rc].get(x - dx, y - dy)
}
