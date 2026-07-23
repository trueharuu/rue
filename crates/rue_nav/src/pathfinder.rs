//! Input path-finding for placements.

#![allow(clippy::many_single_char_names)]

use std::collections::VecDeque;

use rue_core::board::Board;
use rue_core::data::KICKS_I;
use rue_core::data::KICKS_I_180;
use rue_core::data::KICKS_LJSZT;
use rue_core::data::KICKS_LJSZT_180;
use rue_core::data::KICKS_LJSZT_180_JSTRIS;
use rue_core::header::PCELLS;
use rue_core::header::SPAWN_X;
use rue_core::header::SPAWN_Y;
use rue_core::header::WIDTH;
use rue_core::piece::Piece;
use rue_core::placement::Move;
use rue_core::rotation::Rotation;
use rue_core::spin::Spin;
use rue_core::spin::Spins;
use smallvec::SmallVec;

use rue_core::game::ruleset::Ruleset;

/// An individual controller input in a path sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    /// Move one cell left.
    ShiftLeft,
    /// Move one cell right.
    ShiftRight,
    /// Move as far left as possible.
    DasLeft,
    /// Move as far right as possible.
    DasRight,
    /// Rotate clockwise.
    RotateCW,
    /// Rotate counterclockwise.
    RotateCCW,
    /// Rotate 180 degrees.
    RotateFlip,
    /// Move as far down as possible.
    SoftDrop,
    /// Instantly drop to lowest valid position.
    HardDrop,
}

/// A sequence of controller inputs that reach a target placement.
#[derive(Debug)]
pub struct Inputs(pub SmallVec<[Input; 16]>);

/// Board width constant.
const W: usize = WIDTH as usize;

/// Sentinel value for the root path node (no parent).
const ROOT: u16 = u16::MAX;

/// Parent pointer for BFS path reconstruction.
struct PathNode {
    /// The input that led to this node.
    input: Input,
    /// Index of the parent node in the path vector.
    prev: u16,
}

/// State enqueued during BFS.
struct BfsState {
    /// Column position.
    x: i32,
    /// Row position.
    y: i32,
    /// Current rotation.
    r: Rotation,
    /// Accumulated spin label from rotation arrivals.
    s: Spin,
    /// Index of the parent node in the path vector.
    node: u16,
}

/// Returns `true` when placing piece `p` in rotation `r` with origin at `(x, y)`
/// would overlap the board or leave the field.
#[inline]
fn collides<const N: usize>(board: &Board<N>, p: Piece, r: Rotation, x: i32, y: i32) -> bool {
    let h = Board::<N>::H;
    if y < 0 || y >= h || !(0..WIDTH).contains(&x) {
        return true;
    }
    if board.get(x, y) {
        return true;
    }
    let cells = PCELLS[p as usize][r as usize];
    for &cell in &cells {
        let cx = x + i32::from(cell.0);
        let cy = y + i32::from(cell.1);
        if !(0..WIDTH).contains(&cx) || cy < 0 || cy >= h || board.get(cx, cy) {
            return true;
        }
    }
    false
}

/// Returns `true` when all non-origin minos plus the origin are within horizontal bounds.
#[inline]
fn in_bounds(p: Piece, r: Rotation, x: i32) -> bool {
    if !(0..WIDTH).contains(&x) {
        return false;
    }
    let cells = PCELLS[p as usize][r as usize];
    for &cell in &cells {
        let cx = x + i32::from(cell.0);
        if !(0..WIDTH).contains(&cx) {
            return false;
        }
    }
    true
}

/// Drops the piece from `y` as far down as gravity allows.
#[inline]
fn drop_y<const N: usize>(board: &Board<N>, p: Piece, r: Rotation, x: i32, mut y: i32) -> i32 {
    while y > 0 && !collides(board, p, r, x, y - 1) {
        y -= 1;
    }
    y
}

/// T-spin 3-corner detection. Returns `(is_spin, is_full)`.
fn t_spin_3corner<const N: usize>(
    board: &Board<N>,
    x: i32,
    y: i32,
    r: Rotation,
    kick_idx: usize,
) -> (bool, bool) {
    let h = Board::<N>::H;
    let corner = |dx: i32, dy: i32| -> bool {
        let cx = x + dx;
        let cy = y + dy;
        !(0..WIDTH).contains(&cx) || cy < 0 || cy >= h || board.get(cx, cy)
    };
    let nw = corner(-1, 1);
    let ne = corner(1, 1);
    let se = corner(1, -1);
    let sw = corner(-1, -1);
    let spins = (nw && ne && (se || sw)) || (se && sw && (nw || ne));
    if !spins {
        return (false, false);
    }
    if kick_idx >= 4 {
        return (true, true);
    }
    let front = match r {
        Rotation::North => nw && ne,
        Rotation::East => ne && se,
        Rotation::South => se && sw,
        Rotation::West => sw && nw,
    };
    (true, front)
}

/// Returns `true` when the piece cannot shift in any cardinal direction.
fn is_immobile<const N: usize>(board: &Board<N>, p: Piece, r: Rotation, x: i32, y: i32) -> bool {
    (x == 0 || collides(board, p, r, x - 1, y))
        && (x >= WIDTH - 1 || collides(board, p, r, x + 1, y))
        && (y == 0 || collides(board, p, r, x, y - 1))
        && collides(board, p, r, x, y + 1)
}

/// Classifies the spin label after a rotation arrives at `(x, y)` in rotation `r`.
fn classify_spin<const N: usize>(
    board: &Board<N>,
    p: Piece,
    spins: Spins,
    x: i32,
    y: i32,
    r: Rotation,
    kick_idx: usize,
) -> Spin {
    let can_t = p == Piece::T && spins.has_3corner();
    let can_allspin = p != Piece::T && p != Piece::O && spins.has_immobile();

    if can_t {
        let (is_spin, is_full) = t_spin_3corner(board, x, y, r, kick_idx);
        if !is_spin {
            return Spin::None;
        }
        return if is_full { Spin::Full } else { Spin::Mini };
    }
    if can_allspin && is_immobile(board, p, r, x, y) {
        return Spin::Mini;
    }
    Spin::None
}

/// Finds a sequence of inputs that reaches a target placement from the spawn position.
///
/// [`finesse`] determines whether to use DAS or not.
///
/// Returns an empty sequence if the target is unreachable.
#[must_use]
#[inline]
pub fn get_input<const N: usize>(
    board: &Board<N>,
    target: Move,
    ruleset: &Ruleset,
    finesse: bool,
) -> Inputs {
    match target.piece() {
        Piece::T => get_input_impl::<{ Piece::T }, N>(board, target, ruleset, finesse),
        Piece::I => get_input_impl::<{ Piece::I }, N>(board, target, ruleset, finesse),
        Piece::O => get_input_impl::<{ Piece::O }, N>(board, target, ruleset, finesse),
        Piece::L => get_input_impl::<{ Piece::L }, N>(board, target, ruleset, finesse),
        Piece::J => get_input_impl::<{ Piece::J }, N>(board, target, ruleset, finesse),
        Piece::S => get_input_impl::<{ Piece::S }, N>(board, target, ruleset, finesse),
        Piece::Z => get_input_impl::<{ Piece::Z }, N>(board, target, ruleset, finesse),
    }
}

/// Internal specialized implementation over [`Piece`] for [`get_input`].
#[must_use]
#[inline]
pub fn get_input_impl<const P: Piece, const N: usize>(
    board: &Board<N>,
    target: Move,
    ruleset: &Ruleset,
    finesse: bool,
) -> Inputs {
    let spins = ruleset.spins;
    let can_t = P == Piece::T && spins.has_3corner();
    let can_allspin = P != Piece::T && P != Piece::O && spins.has_immobile();
    let can_spin = can_t || can_allspin;
    let spin_nb = if can_spin { Spin::NB } else { 1 };
    let spawn_y = SPAWN_Y.min(Board::<N>::H - 2);

    if collides(board, P, Rotation::North, SPAWN_X, spawn_y) {
        return Inputs(SmallVec::new());
    }

    // Pre-compute the target's cell set for canonical-equivalence matching.
    let target_cells = {
        let tx = target.x();
        let ty = target.y();
        let tr = target.rotation();
        let mut cells = [(tx, ty); 4];
        let offsets = PCELLS[P as usize][tr as usize];
        cells[1] = (tx + i32::from(offsets[0].0), ty + i32::from(offsets[0].1));
        cells[2] = (tx + i32::from(offsets[1].0), ty + i32::from(offsets[1].1));
        cells[3] = (tx + i32::from(offsets[2].0), ty + i32::from(offsets[2].1));
        cells.sort_unstable();
        cells
    };

    // searched[spin][col][rot] — one u64 bitboard of y-positions per cell.
    let mut searched = vec![[[0u64; Rotation::NB]; W]; spin_nb];
    let mut nodes: Vec<PathNode> = Vec::new();
    let mut queue: VecDeque<BfsState> = VecDeque::new();

    searched[0][SPAWN_X as usize][Rotation::North as usize] |= 1u64 << spawn_y;
    queue.push_back(BfsState {
        x: SPAWN_X,
        y: spawn_y,
        r: Rotation::North,
        s: Spin::None,
        node: ROOT,
    });

    while let Some(st) = queue.pop_front() {
        let x = st.x;
        let y = st.y;
        let r = st.r;

        // ---- hard drop (gravity descent) ----
        let dy = drop_y(board, P, r, x, y);
        let sc = if can_spin && dy == y {
            st.s
        } else {
            Spin::None
        };

        if !can_spin || sc == target.spin() {
            let offsets = PCELLS[P as usize][r as usize];
            let mut cells = [(x, dy); 4];
            cells[1] = (x + i32::from(offsets[0].0), dy + i32::from(offsets[0].1));
            cells[2] = (x + i32::from(offsets[1].0), dy + i32::from(offsets[1].1));
            cells[3] = (x + i32::from(offsets[2].0), dy + i32::from(offsets[2].1));
            cells.sort_unstable();
            if cells == target_cells {
                let mut result = SmallVec::<[Input; 16]>::new();
                result.push(Input::HardDrop);
                let mut idx = st.node;
                while idx != ROOT {
                    result.push(nodes[idx as usize].input);
                    idx = nodes[idx as usize].prev;
                }
                result.reverse();
                return Inputs(result);
            }
        }

        // ---- rotations (CW, CCW) ----
        if P != Piece::O {
            for &(cw, inp) in &[(true, Input::RotateCW), (false, Input::RotateCCW)] {
                let rt = if cw { r.cw() } else { r.ccw() };
                let d = usize::from(!cw);

                let off_x = P.canonical_offset(r as usize).0 - P.canonical_offset(rt as usize).0;
                let off_y = P.canonical_offset(r as usize).1 - P.canonical_offset(rt as usize).1;

                let kicks = if matches!(P, Piece::I) {
                    KICKS_I
                } else {
                    KICKS_LJSZT
                };
                let kick_row = kicks[d][r as usize];

                for (k, &(kx, ky)) in kick_row.iter().enumerate() {
                    let x1 = x + i32::from(kx) + off_x;
                    let y1 = y + i32::from(ky) + off_y;

                    if x1 < 0 || y1 < 0 || y1 >= Board::<N>::H {
                        continue;
                    }
                    if !in_bounds(P, rt, x1) {
                        continue;
                    }
                    if collides(board, P, rt, x1, y1) {
                        continue;
                    }

                    let s_new = if can_spin {
                        classify_spin(board, P, spins, x1, y1, rt, k)
                    } else {
                        Spin::None
                    };
                    let si = if can_spin { s_new as usize } else { 0 };

                    if searched[si][x1 as usize][rt as usize] & (1u64 << y1) == 0 {
                        searched[si][x1 as usize][rt as usize] |= 1u64 << y1;
                        let ni = nodes.len() as u16;
                        nodes.push(PathNode {
                            input: inp,
                            prev: st.node,
                        });
                        queue.push_back(BfsState {
                            x: x1,
                            y: y1,
                            r: rt,
                            s: s_new,
                            node: ni,
                        });
                    }
                    break; // first non-colliding kick resolves (SRS)
                }
            }
        }

        // ---- 180 rotation ----
        if P != Piece::O && ruleset.use_180 {
            let rt = r.flip();
            let off_x = P.canonical_offset(r as usize).0 - P.canonical_offset(rt as usize).0;
            let off_y = P.canonical_offset(r as usize).1 - P.canonical_offset(rt as usize).1;

            let kick_row = if matches!(P, Piece::I) {
                KICKS_I_180[r as usize]
            } else if ruleset.srs_plus {
                KICKS_LJSZT_180[r as usize]
            } else {
                KICKS_LJSZT_180_JSTRIS[r as usize]
            };

            for (k, &(kx, ky)) in kick_row.iter().enumerate() {
                let x1 = x + i32::from(kx) + off_x;
                let y1 = y + i32::from(ky) + off_y;

                if x1 < 0 || y1 < 0 || y1 >= Board::<N>::H {
                    continue;
                }
                if !in_bounds(P, rt, x1) {
                    continue;
                }
                if collides(board, P, rt, x1, y1) {
                    continue;
                }

                let s_new = if can_spin {
                    classify_spin(board, P, spins, x1, y1, rt, k)
                } else {
                    Spin::None
                };
                let si = if can_spin { s_new as usize } else { 0 };

                if searched[si][x1 as usize][rt as usize] & (1u64 << y1) == 0 {
                    searched[si][x1 as usize][rt as usize] |= 1u64 << y1;
                    let ni = nodes.len() as u16;
                    nodes.push(PathNode {
                        input: Input::RotateFlip,
                        prev: st.node,
                    });
                    queue.push_back(BfsState {
                        x: x1,
                        y: y1,
                        r: rt,
                        s: s_new,
                        node: ni,
                    });
                }
                break; // first non-colliding kick resolves
            }
        }

        // ---- shift left / right ----
        for &(dx, inp) in &[(-1i32, Input::ShiftLeft), (1, Input::ShiftRight)] {
            let x1 = x + dx;
            if !in_bounds(P, r, x1) || collides(board, P, r, x1, y) {
                continue;
            }
            let si = if can_spin { Spin::None as usize } else { 0 };
            if searched[si][x1 as usize][r as usize] & (1u64 << y) != 0 {
                continue;
            }
            searched[si][x1 as usize][r as usize] |= 1u64 << y;
            let ni = nodes.len() as u16;
            nodes.push(PathNode {
                input: inp,
                prev: st.node,
            });
            queue.push_back(BfsState {
                x: x1,
                y,
                r,
                s: Spin::None,
                node: ni,
            });
        }

        // ---- DAS left / right ----
        if finesse {
            for &(dx, inp) in &[(-1i32, Input::DasLeft), (1, Input::DasRight)] {
                let mut x1 = x + dx;
                loop {
                    if x1 < 0 || !in_bounds(P, r, x1) || collides(board, P, r, x1, y) {
                        break;
                    }
                    x1 += dx;
                }
                x1 -= dx; // back to last valid position
                if x1 == x {
                    continue;
                }
                let si = if can_spin { Spin::None as usize } else { 0 };
                if searched[si][x1 as usize][r as usize] & (1u64 << y) != 0 {
                    continue;
                }
                searched[si][x1 as usize][r as usize] |= 1u64 << y;
                let ni = nodes.len() as u16;
                nodes.push(PathNode {
                    input: inp,
                    prev: st.node,
                });
                queue.push_back(BfsState {
                    x: x1,
                    y,
                    r,
                    s: Spin::None,
                    node: ni,
                });
            }
        }

        // ---- soft drop (drop as far as possible) ----
        {
            let ny = drop_y(board, P, r, x, y);
            if ny < y {
                let si = if can_spin { Spin::None as usize } else { 0 };
                if searched[si][x as usize][r as usize] & (1u64 << ny) == 0 {
                    searched[si][x as usize][r as usize] |= 1u64 << ny;
                    let ni = nodes.len() as u16;
                    nodes.push(PathNode {
                        input: Input::SoftDrop,
                        prev: st.node,
                    });
                    queue.push_back(BfsState {
                        x,
                        y: ny,
                        r,
                        s: Spin::None,
                        node: ni,
                    });
                }
            }
        }
    }

    Inputs(SmallVec::new())
}

#[cfg(test)]
#[allow(clippy::unreadable_literal)]
mod tests {
    use crate::pathfinder;
    use rue_core::board::Board;
    use rue_core::game::ruleset::SEASON_2;
    use rue_core::placement::Move;

    // failed because couldn't 180 inplace? i think?
    // also, reaching something that is canonically equal to the target should be valid
    // because they fill the same cells and are equivalent when hard-dropped
    #[test]
    fn fail_1() {
        let board = Board::from_vector([861810856984383, 0, 0, 0, 0, 0, 0, 0].into());
        let mv = unsafe { Move::from_raw(2734759936) };
        let inputs = pathfinder::get_input(&board, mv, &SEASON_2, true);
        assert!(!inputs.0.is_empty());
    }

    // somehow the same exact reason?
    #[test]
    fn fail_2() {
        let board = Board::from_vector([426241851327, 0, 0, 0, 0, 0, 0, 0].into());
        let mv = unsafe { Move::from_raw(2860556288) };
        let inputs = pathfinder::get_input(&board, mv, &SEASON_2, true);
        assert!(!inputs.0.is_empty());
    }

    // same thing again, path leads to a west-facing Z piece in the same spot but we're expecting east
    #[test]
    fn fail_3() {
        let board = Board::from_vector([1031745281991, 0, 0, 0, 0, 0, 0, 0].into());
        let mv = unsafe { Move::from_raw(3380649984) };
        let inputs = pathfinder::get_input(&board, mv, &SEASON_2, true);
        assert!(!inputs.0.is_empty());
    }

    // same thing but with I piece
    #[test]
    fn fail_4() {
        let board = Board::from_vector([847488152330288639, 768, 0, 0, 0, 0, 0, 0].into());
        let mv = unsafe { Move::from_raw(704749568) };
        let inputs = pathfinder::get_input(&board, mv, &SEASON_2, true);
        assert!(!inputs.0.is_empty());
    }

    // new case, immobile non-3-corner T spin mini seems to not work
    #[test]
    fn fail_5() {
        let board = Board::from_vector([3250470463, 0, 0, 0, 0, 0, 0, 0].into());
        let mv = unsafe { Move::from_raw(41984000) };
        let inputs = pathfinder::get_input(&board, mv, &SEASON_2, true);
        assert!(!inputs.0.is_empty());
    }

    // immobile, 3-corner T spin mini pointed up, this probably has to do with spin provenance
    #[test]
    fn fail_6() {
        let board = Board::from_vector([103184014959, 0, 0, 0, 0, 0, 0, 0].into());
        let mv = unsafe { Move::from_raw(33587200) };
        let inputs = pathfinder::get_input(&board, mv, &SEASON_2, true);
        assert!(!inputs.0.is_empty());
    }

    // regular t-spin single.
    // can't find `None` placement because it's expecting `Full`
    #[test]
    fn fail_7() {
        let board = Board::from_vector([66879423, 0, 0, 0, 0, 0, 0, 0].into());
        let mv = unsafe { Move::from_raw(453017600) };
        let inputs = pathfinder::get_input(&board, mv, &SEASON_2, true);
        assert!(!inputs.0.is_empty());
    }

    // i believe this requires a 180 to get to. same as before, expecting Mini but we never emit it
    #[test]
    fn fail_8() {
        let board = Board::from_vector([563941838749631, 0, 0, 0, 0, 0, 0, 0].into());
        let mv = unsafe { Move::from_raw(58761216) };
        let inputs = pathfinder::get_input(&board, mv, &SEASON_2, true);
        assert!(!inputs.0.is_empty());
    }

    // this requires non-infinite soft drop. this is just straight up not supported.
    // this test is mostly a signal for a movegen "fix".
    //
    // from the future: this fix has been applied. hooray!
    // #[test]
    // fn fail_9_expected() {
    //     let board = Board::from_vector([15839586959247, 0, 0, 0, 0, 0, 0, 0].into());
    //     let mv = unsafe { Move::from_raw(2197848064) };
    //     let inputs = pathfinder::get_input(&board, mv, &SEASON_2, true);
    //     assert!(inputs.0.is_empty());
    // }
}
