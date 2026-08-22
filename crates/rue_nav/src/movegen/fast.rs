use rue_core::board::Board;
use rue_core::data::KICKS_I;
use rue_core::data::KICKS_O;
use rue_core::data::KICKS_TJLSZ;
use rue_core::envelope::EnvelopeTable;
use rue_core::envelope::env_probe;
use rue_core::header::WIDTH;
use rue_core::piece::Piece;
use rue_core::rot_idx;
use rue_core::rule::Rule;
use rue_core::spin::Spins;

use crate::buffer::Moves;
use crate::movegen::op::horizontal_tuck;
use crate::movegen::op::landable_map;
use crate::movegen::op::usable_map;
use crate::movegen::op::vertical_ceiling;
use crate::movegen::op::vertical_drop;
use crate::unroll;

/// Generates all reachable landed positions for a given piece on a given board,
/// under the specified rule set.
#[inline]
#[must_use]
pub fn movegen<const N: usize, const RULE: Rule>(board: &Board<N>, piece: Piece, y: i32, force: i32) -> Moves<N> {
    match piece {
        Piece::T => generate_inlined::<N, { Piece::T }, RULE, true>(board, y, force),
        Piece::I => generate_inlined::<N, { Piece::I }, RULE, true>(board, y, force),
        Piece::J => generate_inlined::<N, { Piece::J }, RULE, true>(board, y, force),
        Piece::L => generate_inlined::<N, { Piece::L }, RULE, true>(board, y, force),
        Piece::O => generate_inlined::<N, { Piece::O }, RULE, true>(board, y, force),
        Piece::S => generate_inlined::<N, { Piece::S }, RULE, true>(board, y, force),
        Piece::Z => generate_inlined::<N, { Piece::Z }, RULE, true>(board, y, force),
    }
    .0
}

/// Counts the number of reachable landed positions for a single piece and rule
/// on the given board.
#[inline]
#[must_use]
pub fn count_locks<const N: usize, const P: Piece, const RULE: Rule>(board: &Board<N>, y: i32, force: i32) -> u64 {
    generate_inlined::<N, P, RULE, false>(board, y, force).1
}

// Generates reachable/landable placements for piece `P` on board `b`.
///
/// When `EMIT` is `true`, returns populated move buckets and a zero count. When
/// `EMIT` is `false`, returns empty buckets and the number of reachable
/// landable placements.
#[inline]
#[must_use]
pub fn generate_inlined<const N: usize, const P: Piece, const RULE: Rule, const EMIT: bool>(
    b: &Board<N>,
    y: i32,
    force: i32,
) -> (Moves<N>, u64) {
    let h = Board::<N>::total_height();
    let usable = usable_map::<N, P>(b);
    let cs = P.groups();
    let ss = P.search_size();
    let all_done = (1u64 << P.search_size()) - 1;
    let cands = landable_map(&usable, P.groups());
    let mut missing = [Board::empty(); 4];
    let mut search = [Board::empty(); 4];
    let mut unsearched = [Board::empty(); 4];
    let mut remaining = 0;
    let mut done;
    let mut total = 0;

    {
        if !EMIT {
            unroll!(r, cs, {
                total += cands[r].popcount();
            });
        }

        if h > RULE.spawn_y && y > RULE.spawn_y - P.h_spawn() {
            let threshold = (RULE.spawn_y + force + 1).min(h);

            let mut spawn_y = RULE.spawn_y;
            while spawn_y < threshold && !usable[0].get(RULE.spawn_x, spawn_y) {
                spawn_y += 1;
            }

            if spawn_y == threshold {
                return (Moves::empty(P), 0);
            }

            search[0].set(RULE.spawn_x, spawn_y);

            unroll!(r, cs, {
                missing[r] = cands[r];
                if missing[r].any() {
                    remaining |= 1 << r;
                }
            });

            done = all_done & !1;
        } else {
            let ceiling = h - P.h_gen();

            unroll!(r, cs, {
                let surface = vertical_ceiling(!usable[r], ceiling);
                search[r] = if const { RULE.inf_sdf } {
                    !surface & cands[r]
                } else {
                    !surface
                };
                missing[r] = cands[r] & !search[r];

                if missing[r].any() {
                    remaining |= 1 << r;
                }
            });

            if remaining == 0 {
                return finish::<N, P, RULE, EMIT>(cs, &cands, &missing, remaining, total);
            }

            // two rounds of horizontal tucks
            unroll!(r, cs, {
                let mut s = search[r];
                s = horizontal_tuck(s, &usable[r]);
                s = horizontal_tuck(s, &usable[r]);
                search[r] = s;
            });

            if P.group4() {
                unroll!(r, 4, {
                    search[r] |= (search[(r + 1) & 3] | search[(r + 3) & 3]) & usable[r];
                });
            }

            remaining = 0;

            unroll!(r, cs, {
                missing[r] &= !search[r];

                if missing[r].any() {
                    remaining |= 1 << r;
                }
            });

            if remaining == 0 {
                return finish::<N, P, RULE, EMIT>(cs, &cands, &missing, remaining, total);
            }

            if P.group2() {
                search[2] = search[0];
                search[3] = search[1];
            }

            done = 0;
        }
    }

    unroll!(r, ss, {
        unsearched[r] = !search[r] & usable[const { P.canonical_rotation(rot_idx!(r)) as usize }];
    });

    while done != all_done {
        unroll!(r, 4, {
            process_rot::<N, P, RULE, r>(
                ss,
                &mut done,
                &mut search,
                &mut unsearched,
                &mut missing,
                &mut remaining,
                &usable,
                all_done,
            );
        });
    }

    finish::<N, P, RULE, EMIT>(cs, &cands, &missing, remaining, total)
}

#[inline]
fn finish<const N: usize, const P: Piece, const RULE: Rule, const EMIT: bool>(
    cs: usize,
    cands: &[Board<N>; 4],
    missing: &[Board<N>; 4],
    remaining: u64,
    total: u64,
) -> (Moves<N>, u64) {
    if EMIT {
        let mut moves = Moves::empty(P);

        unroll!(r, cs, {
            let landable = cands[r] ^ missing[r];

            moves.none[r] = landable;
        });

        return (moves, 0);
    }

    let mut miss = 0;
    unroll!(r, 4, {
        if remaining & (1 << r) != 0 {
            miss += missing[r].popcount();
        }
    });

    (Moves::empty(P), total - miss)
}

#[inline]
fn process_rot<const N: usize, const P: Piece, const RULE: Rule, const R: usize>(
    ss: usize,
    done: &mut u64,
    search: &mut [Board<N>; 4],
    unsearched: &mut [Board<N>; 4],
    missing: &mut [Board<N>; 4],
    remaining: &mut u64,
    usable: &[Board<N>; 4],
    all_done: u64,
) {
    if R < ss && *done & (1 << R) == 0 {
        *done |= 1 << R;
        let rc: usize = P.canonical_rotation(rot_idx!(R)) as usize;

        loop {
            let temp_all = search[R].shifted(-1, 0)
                | search[R].shifted(1, 0)
                | vertical_drop::<N, RULE>(search[R], &usable[rc]);

            let temp = temp_all & unsearched[R];

            if !temp.any() {
                break;
            }

            search[R] |= temp;
            unsearched[R] ^= temp;
        }

        missing[rc] &= !search[R];

        if missing[rc].any() {
            *remaining |= 1 << rc;
        } else {
            *remaining &= !(1 << rc);
        }

        if *remaining == 0 {
            *done = all_done;
        } else {
            if !matches!(P, Piece::O) {
                let probe = env_probe(&search[R], EnvelopeTable::<P, R>::E);

                // rotation direction 0/1 (cw/ccw), 6 kicks (indicies 0-5)
                rot_kick_seq::<N, P, RULE, R, 0>(probe, search, unsearched, missing, done, remaining, usable);
                rot_kick_seq::<N, P, RULE, R, 1>(probe, search, unsearched, missing, done, remaining, usable);

                // rotation direction 2 (180)
                if const { RULE.allow_180 } {
                    rot_kick_seq::<N, P, RULE, R, 2>(probe, search, unsearched, missing, done, remaining, usable);
                }

                if *remaining == 0 {
                    *done = all_done;
                }
            }

            if *done != all_done {
                search[R] = Board::empty();
            }
        }
    }
}

#[inline]
fn rot_kick_seq<const N: usize, const P: Piece, const RULE: Rule, const R: usize, const D: usize>(
    probe: Board<N>,
    search: &mut [Board<N>; 4],
    unsearched: &mut [Board<N>; 4],
    missing: &mut [Board<N>; 4],
    done: &mut u64,
    remaining: &mut u64,
    usable: &[Board<N>; 4],
) {
    if !probe.any() {
        return;
    }

    let kt = match P {
        Piece::I => &KICKS_I,
        Piece::O => &KICKS_O,
        _ => &KICKS_TJLSZ,
    };

    let r1 = if D == 0 {
        (R + 1) & 3
    } else if D == 1 {
        (R + 3) & 3
    } else {
        (R + 2) & 3
    };
    let r1c = P.canonical_rotation(rot_idx!(r1)) as usize;

    let off_x = P.canonical_offset(rot_idx!(R)).0 - P.canonical_offset(rot_idx!(r1)).0;
    let off_y = P.canonical_offset(rot_idx!(R)).1 - P.canonical_offset(rot_idx!(r1)).1;

    let mut temp = search[R];

    macro_rules! step {
        ($n:literal) => {{
            let lane = kt[R][r1];
            if $n >= lane.1 {
                return;
            }

            let kx = i32::from(lane.0[$n].0) + off_x;
            let ky = i32::from(lane.0[$n].1) + off_y;

            let res = temp.shifted(kx, ky) & unsearched[r1];

            if res.any() {
                search[r1] |= res;
                unsearched[r1] ^= res;
                *done &= !(1 << r1);
                missing[r1c] &= !res;

                if missing[r1c].any() {
                    *remaining |= 1 << r1c;
                } else {
                    *remaining ^= 1 << r1c;
                }
            }

            temp &= !usable[r1c].shifted(-kx, -ky);
        }};
    }

    step!(0);
    step!(1);
    step!(2);
    step!(3);
    step!(4);
    step!(5);
}
