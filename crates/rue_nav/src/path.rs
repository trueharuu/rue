use rue_core::board::Board;
use rue_core::buffer::Buffer;
use rue_core::piece::Piece;
use rue_core::placement::Move;
use rue_core::rotation::Rotation;
use rue_core::rule::Rule;
use rue_core::spin::Spin;

use crate::buffer::Moves;
use crate::movegen::op::apply_rotation;
use crate::movegen::op::check_fast;
use crate::movegen::op::usable_map;
use crate::movegen::queue::Queue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    DasLeft,
    DasRight,
    TapLeft,
    TapRight,
    SoftDrop,
    RotateCW,
    RotateCCW,
    Rotate180,
    HardDrop,
}

#[inline]
#[must_use]
pub fn generate_inlined<const N: usize, const P: Piece, const RULE: Rule>(
    board: &Board<N>,
    y: i32,
    force: i32,
    target: Move,
) -> Buffer<Key, 16> {
    let mut queue = Queue::new();
    let mut visited: Moves<N> = Moves::empty(P);

    let usable = usable_map::<N, P>(board);

    // `y` is the drop height (top of the stack).
    // When it rises within `h_spawn` rows of the spawn row, a piece at the
    // nominal spawn overlaps the stack. The spawn then rises to the first free
    // cell in the spawn column. The scan is bounded by `force` rows of allowed
    // rise plus the field top. If the bound has no free cell, the piece is
    // locked out and no placement is reachable.
    let sx = RULE.spawn_x;
    // The nominal spawn row is defined for a full-height field. Banded boards
    // (for example, the band-cast perft driver) can be shorter than the spawn
    // row. Clamp the spawn to the highest row where the piece fits in its spawn
    // orientation. The BFS then explores the same landed set that a top-edge
    // spawn produces.
    let top = Board::<N>::total_height();
    let sy = RULE.spawn_y;

    let mut spawn_row = sy;
    if y > sy - P.h_spawn() {
        let threshold = (sy + force + 1).min(top);
        while spawn_row < threshold
            && !check_fast::<N, P>(&usable, sx, spawn_row, Rotation::North as usize)
        {
            spawn_row += 1;
        }

        if spawn_row == threshold {
            return Buffer::new();
        }
    }

    let spawn = Move::new(P, sx, spawn_row, Rotation::North, Spin::None);
    if check_fast::<N, P>(&usable, spawn.x(), spawn.y(), spawn.rotation() as usize) {
        queue.push_back((Buffer::new(), spawn));
    }
    while let Some((path, ghost)) = queue.pop_front() {
        if !visited.insert(ghost) {
            continue;
        }

        let x = ghost.x();
        let y = ghost.y();
        let r = ghost.rotation() as usize;

        // hard drop
        let mut drop_y = y;
        while check_fast::<N, P>(&usable, x, drop_y - 1, r) {
            drop_y -= 1;
        }

        {
            let dropped_ghost = Move::new(P, x, drop_y, ghost.rotation(), {
                if drop_y == y {
                    ghost.spin()
                } else {
                    Spin::None
                }
            });

            // return path if the target is reached
            if dropped_ghost.canonicalize() == target.canonicalize() {
                let mut result = path;
                result.push(Key::HardDrop);
                return result;
            }
        }

        // extended lateral movement (das)
        if RULE.das {
            'd: for dx in [-1, 1] {
                let mut x1 = x;
                loop {
                    let next = x1 + dx;
                    if !check_fast::<N, P>(&usable, next, y, r) {
                        break;
                    }
                    x1 = next;
                }

                if x1 == x {
                    continue 'd;
                }

                let new_ghost = Move::new(P, x1, y, ghost.rotation(), Spin::None);
                let mut new_path = path;
                new_path.push(if dx < 0 { Key::DasLeft } else { Key::DasRight });
                queue.push_back((new_path, new_ghost));
            }
        }

        // lateral movement
        'd: for dx in [-1, 1] {
            let x1 = x + dx;
            if !check_fast::<N, P>(&usable, x1, y, r) {
                continue 'd;
            }

            let new_ghost = Move::new(ghost.piece(), x1, y, ghost.rotation(), Spin::None);
            let mut new_path = path;
            new_path.push(if dx < 0 { Key::TapLeft } else { Key::TapRight });
            queue.push_back((new_path, new_ghost));
        }

        // soft drop
        {
            let mut dy = y;
            if RULE.inf_sdf {
                while check_fast::<N, P>(&usable, x, dy - 1, r) {
                    dy -= 1;
                }
            } else if check_fast::<N, P>(&usable, x, dy - 1, r) {
                dy -= 1;
            }

            if dy != y {
                let new_ghost = Move::new(ghost.piece(), x, dy, ghost.rotation(), Spin::None);
                let mut new_path = path;
                new_path.push(Key::SoftDrop);
                queue.push_back((new_path, new_ghost));
            }
        }

        // rotation (cw)
        {
            let new_ghost =
                apply_rotation::<N, P, RULE>(board, &usable, &ghost, ghost.rotation().cw());
            if new_ghost != ghost {
                let mut new_path = path;
                new_path.push(Key::RotateCW);
                queue.push_back((new_path, new_ghost));
            }
        }

        // rotation (ccw)
        {
            let new_ghost =
                apply_rotation::<N, P, RULE>(board, &usable, &ghost, ghost.rotation().ccw());
            if new_ghost != ghost {
                let mut new_path = path;
                new_path.push(Key::RotateCCW);
                queue.push_back((new_path, new_ghost));
            }
        }

        // rotation (180)
        if RULE.allow_180 {
            let new_ghost =
                apply_rotation::<N, P, RULE>(board, &usable, &ghost, ghost.rotation().cw().cw());
            if new_ghost != ghost {
                let mut new_path = path;
                new_path.push(Key::Rotate180);
                queue.push_back((new_path, new_ghost));
            }
        }
    }

    Buffer::new()
}

#[cfg(test)]
mod tests {
    use rue_core::board::Board;
    use rue_core::piece::Piece;
    use rue_core::rule::DEFAULT;

    use super::generate_inlined;
    use crate::movegen::oracle;

    fn check<const P: Piece>() {
        let board = Board::<4>::empty();
        let landed = oracle::generate_inlined::<4, P, DEFAULT>(&board, 0, 0).0;

        for target in &landed {
            assert!(
                !generate_inlined::<4, P, DEFAULT>(&board, 0, 0, target).is_empty(),
                "no path to {target:?}",
            );
        }
    }

    #[test]
    fn every_reachable_placement_has_a_path() {
        check::<{ Piece::I }>();
        check::<{ Piece::J }>();
        check::<{ Piece::L }>();
        check::<{ Piece::O }>();
        check::<{ Piece::S }>();
        check::<{ Piece::T }>();
        check::<{ Piece::Z }>();
    }
}
