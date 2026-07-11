//! Input path-finding for placements.

use std::collections::VecDeque;

use smallvec::SmallVec;

use rue_core::{
    board::Board,
    data,
    game::ruleset::Ruleset,
    header::{SPAWN_X, SPAWN_Y, WIDTH},
    piece::Piece,
    placement::Move,
    spin::Spins,
};

use crate::collision;

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
    /// Move one cell down.
    SoftDrop,
    /// Instantly drop to lowest valid position.
    HardDrop,
    /// [`HardDrop`], without locking the piece.
    SonicDrop,
}

/// A sequence of controller inputs that reach a target placement.
#[derive(Debug)]
pub struct Inputs(pub SmallVec<[Input; 16]>);

const ROOT: u32 = u32::MAX;

struct PathNode {
    input: Input,
    prev: u32,
}

#[derive(Clone, Copy)]
struct GhostMove {
    r: usize,
    x: i32,
    y: i32,
    prev: u32,
    spin: usize,
}

#[inline]
fn is_ok_x(x: i32) -> bool {
    (0..WIDTH).contains(&x)
}

#[inline]
fn is_ok_y(y: i32, h: i32) -> bool {
    (0..h).contains(&y)
}

fn pathfind_impl<const P: Piece, const N: usize>(
    board: &Board<N>,
    target: Move,
    ruleset: &Ruleset,
    finesse: bool,
    force: bool,
) -> Inputs {
    let check_tspin = P == Piece::T && !matches!(ruleset.spins, Spins::None);
    let spin_mul = if check_tspin { 3 } else { 1 };
    let h = Board::<N>::H;

    let usable = collision::usable_map::<P, N>(board);

    let force_val = i32::from(force);
    let threshold = (SPAWN_Y + force_val + 1).min(h);
    let mut spawn = SPAWN_Y;
    while spawn < threshold && !usable[0].get(SPAWN_X, spawn) {
        spawn += 1;
    }
    if spawn >= threshold {
        return Inputs(SmallVec::new());
    }

    let target_r = P.canonical_rotation(target.rotation() as usize);
    let target_x = target.x();
    let target_y = target.y();
    let target_spin = if check_tspin {
        target.spin() as usize
    } else {
        0
    };

    let mut searched = vec![Board::<N>::EMPTY; spin_mul * 4];
    let mut leaf: VecDeque<GhostMove> = VecDeque::new();
    let mut internal: Vec<PathNode> = Vec::new();

    let spin_index = |s: usize| -> usize { if check_tspin { s } else { 0 } };

    let start_idx = spin_index(0) * 4;
    searched[start_idx].set(SPAWN_X, spawn);
    leaf.push_back(GhostMove {
        r: 0,
        x: SPAWN_X,
        y: spawn,
        prev: ROOT,
        spin: 0,
    });

    while let Some(m) = leaf.pop_front() {
        let mut l = m;
        while is_ok_y(l.y - 1, h) && usable[P.canonical_rotation(l.r)].get(l.x, l.y - 1) {
            l.y -= 1;
        }

        if check_tspin && l.y != m.y {
            l.spin = 0;
        }

        if P.canonical_rotation(l.r) == target_r
            && l.x == target_x
            && l.y == target_y
            && l.spin == target_spin
        {
            let mut inputs = SmallVec::new();
            let mut i = l.prev;
            while i != ROOT {
                inputs.push(internal[i as usize].input);
                i = internal[i as usize].prev;
            }
            inputs.reverse();
            inputs.push(Input::HardDrop);
            return Inputs(inputs);
        }

        let spin = if check_tspin { 0 } else { m.spin };

        if P != Piece::O {
            for &(d, input) in &[(0usize, Input::RotateCW), (1usize, Input::RotateCCW)] {
                let r1 = if d == 0 { (m.r + 1) & 3 } else { (m.r + 3) & 3 };
                let r1c = P.canonical_rotation(r1);
                let off_x = P.canonical_offset(m.r).0 - P.canonical_offset(r1).0;
                let off_y = P.canonical_offset(m.r).1 - P.canonical_offset(r1).1;
                let kicks = data::kick_row_const(P, d, m.r);

                for (kick_idx, &kick) in kicks.iter().enumerate() {
                    let lx = m.x + i32::from(kick.0) + off_x;
                    let ly = m.y + i32::from(kick.1) + off_y;

                    if !is_ok_x(lx) || !is_ok_y(ly, h) {
                        continue;
                    }

                    if !usable[r1c].get(lx, ly) {
                        continue;
                    }

                    let mut new_spin = spin;
                    if check_tspin {
                        let obstructed = |x: i32, y: i32| -> bool {
                            !is_ok_x(x) || !is_ok_y(y, h) || board.get(x, y)
                        };
                        let corners = [
                            obstructed(lx - 1, ly + 1),
                            obstructed(lx + 1, ly + 1),
                            obstructed(lx + 1, ly - 1),
                            obstructed(lx - 1, ly - 1),
                        ];
                        let count = corners.iter().filter(|&&c| c).count();
                        if count >= 3 {
                            let front_0 = corners[r1];
                            let front_1 = corners[(r1 + 1) % 4];
                            new_spin = if kick_idx >= 4 || (front_0 && front_1) {
                                2
                            } else {
                                1
                            };
                        } else {
                            new_spin = 0;
                        }
                    }

                    let entry_idx = spin_index(new_spin) * 4 + r1;
                    if !searched[entry_idx].get(lx, ly) {
                        searched[entry_idx].set(lx, ly);
                        let prev = internal.len() as u32;
                        internal.push(PathNode {
                            input,
                            prev: m.prev,
                        });
                        leaf.push_back(GhostMove {
                            r: r1,
                            x: lx,
                            y: ly,
                            prev,
                            spin: new_spin,
                        });
                    }
                }
            }
        }

        for &(dx, input) in &[(-1i32, Input::ShiftLeft), (1i32, Input::ShiftRight)] {
            let lx = m.x + dx;

            if !is_ok_x(lx) {
                continue;
            }

            let rc = P.canonical_rotation(m.r);
            if usable[rc].get(lx, m.y) {
                let entry_idx = spin_index(spin) * 4 + m.r;
                if !searched[entry_idx].get(lx, m.y) {
                    searched[entry_idx].set(lx, m.y);
                    let prev = internal.len() as u32;
                    internal.push(PathNode {
                        input,
                        prev: m.prev,
                    });
                    leaf.push_back(GhostMove {
                        r: m.r,
                        x: lx,
                        y: m.y,
                        prev,
                        spin,
                    });
                }

                if finesse {
                    let mut das_x = lx;
                    while is_ok_x(das_x + dx) && usable[rc].get(das_x + dx, m.y) {
                        das_x += dx;
                    }
                    if das_x != lx {
                        let das_input = if dx == 1 {
                            Input::DasRight
                        } else {
                            Input::DasLeft
                        };
                        let entry_idx = spin_index(spin) * 4 + m.r;
                        if !searched[entry_idx].get(das_x, m.y) {
                            searched[entry_idx].set(das_x, m.y);
                            let prev = internal.len() as u32;
                            internal.push(PathNode {
                                input: das_input,
                                prev: m.prev,
                            });
                            leaf.push_back(GhostMove {
                                r: m.r,
                                x: das_x,
                                y: m.y,
                                prev,
                                spin,
                            });
                        }
                    }
                }
            }
        }

        {
            let ly = m.y - 1;
            if is_ok_y(ly, h) && usable[P.canonical_rotation(m.r)].get(m.x, ly) {
                let entry_idx = spin_index(spin) * 4 + m.r;
                if !searched[entry_idx].get(m.x, ly) {
                    searched[entry_idx].set(m.x, ly);
                    let prev = internal.len() as u32;
                    internal.push(PathNode {
                        input: Input::SoftDrop,
                        prev: m.prev,
                    });
                    leaf.push_back(GhostMove {
                        r: m.r,
                        x: m.x,
                        y: ly,
                        prev,
                        spin,
                    });
                }
            }
        }

        // sonic drop
        {
            let mut ly = m.y - 1;
            while is_ok_y(ly, h) && usable[P.canonical_rotation(m.r)].get(m.x, ly) {
                ly -= 1;
            }

            if ly != m.y - 1 {
                let entry_idx = spin_index(spin) * 4 + m.r;
                if !searched[entry_idx].get(m.x, ly + 1) {
                    searched[entry_idx].set(m.x, ly + 1);
                    let prev = internal.len() as u32;
                    internal.push(PathNode {
                        input: Input::SonicDrop,
                        prev: m.prev,
                    });
                    leaf.push_back(GhostMove {
                        r: m.r,
                        x: m.x,
                        y: ly + 1,
                        prev,
                        spin,
                    });
                }
            }
        }
    }

    Inputs(SmallVec::new())
}

/// Finds a sequence of inputs that reaches a target placement from the spawn position.
///
/// Returns an empty sequence if the target is unreachable.
#[must_use]
pub fn get_input<const N: usize>(
    board: &Board<N>,
    target: Move,
    ruleset: &Ruleset,
    finesse: bool,
    force: bool,
) -> Inputs {
    match target.piece() {
        Piece::T => pathfind_impl::<{ Piece::T }, N>(board, target, ruleset, finesse, force),
        Piece::I => pathfind_impl::<{ Piece::I }, N>(board, target, ruleset, finesse, force),
        Piece::J => pathfind_impl::<{ Piece::J }, N>(board, target, ruleset, finesse, force),
        Piece::L => pathfind_impl::<{ Piece::L }, N>(board, target, ruleset, finesse, force),
        Piece::O => pathfind_impl::<{ Piece::O }, N>(board, target, ruleset, finesse, force),
        Piece::S => pathfind_impl::<{ Piece::S }, N>(board, target, ruleset, finesse, force),
        Piece::Z => pathfind_impl::<{ Piece::Z }, N>(board, target, ruleset, finesse, force),
    }
}
