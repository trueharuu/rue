use std::simd::Simd;

use rue_core::board::Board;
use rue_core::data::KICKS_I;
use rue_core::data::KICKS_O;
use rue_core::data::KICKS_TJLSZ;
use rue_core::envelope::EnvelopeTable;
use rue_core::envelope::env_probe;
use rue_core::header::COL0;
use rue_core::header::COL9;
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
        Piece::T => generate_inlined::<N, { Piece::T }, RULE, true>(board, y, force).0,
        Piece::I => generate_inlined::<N, { Piece::I }, RULE, true>(board, y, force).0,
        Piece::J => generate_inlined::<N, { Piece::J }, RULE, true>(board, y, force).0,
        Piece::L => generate_inlined::<N, { Piece::L }, RULE, true>(board, y, force).0,
        Piece::O => generate_inlined::<N, { Piece::O }, RULE, true>(board, y, force).0,
        Piece::S => generate_inlined::<N, { Piece::S }, RULE, true>(board, y, force).0,
        Piece::Z => generate_inlined::<N, { Piece::Z }, RULE, true>(board, y, force).0,
    }
}

/// Counts the number of reachable landed positions for a single piece and rule
/// on the given board.
#[inline]
#[must_use]
pub fn count_locks<const N: usize, const P: Piece, const RULE: Rule>(board: &Board<N>, y: i32, force: i32) -> u64 {
    generate_inlined::<N, P, RULE, false>(board, y, force).1
}

/// Generates reachable and landable placements for piece `P` on board `b`.
///
/// When `EMIT` is `true`, returns populated move buckets and a zero count.
/// When `EMIT` is `false`, returns empty buckets and the number of reachable
/// landable placements.
#[inline]
#[must_use]
pub fn generate_inlined<const N: usize, const P: Piece, const RULE: Rule, const EMIT: bool>(
    board: &Board<N>,
    y: i32,
    force: i32,
) -> (Moves<N>, u64, [Board<N>; 4], [Board<N>; 4], [Board<N>; 4]) {
    let h = Board::<N>::total_height();
    let usable = usable_map::<N, P>(board);
    let cs = P.groups();
    let ss = P.search_size();
    let all_done = (1u64 << P.search_size()) - 1;
    let cands = landable_map(&usable, P.groups());
    let track = EMIT && !matches!(RULE.spins, Spins::None);
    let mut missing = [Board::empty(); 4];
    let mut search = [Board::empty(); 4];
    let mut unsearched = [Board::empty(); 4];
    let mut kicked = [Board::empty(); 4];
    let mut kicked_hi = [Board::empty(); 4];
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
                return (Moves::empty(P), 0, search, kicked, kicked_hi);
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

            if remaining == 0 && !track {
                let (m, c) = finish::<N, P, RULE, EMIT>(cs, board, &usable, &cands, &missing, &kicked, &kicked_hi, remaining, total);
                return (m, c, search, kicked, kicked_hi);
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

            if remaining == 0 && !track {
                let (m, c) = finish::<N, P, RULE, EMIT>(cs, board, &usable, &cands, &missing, &kicked, &kicked_hi, remaining, total);
                return (m, c, search, kicked, kicked_hi);
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
                &mut kicked,
                &mut kicked_hi,
                track,
                all_done,
            );
        });
    }

    let (m, c) = finish::<N, P, RULE, EMIT>(cs, board, &usable, &cands, &missing, &kicked, &kicked_hi, remaining, total);
    (m, c, search, kicked, kicked_hi)
}

#[inline]
fn finish<const N: usize, const P: Piece, const RULE: Rule, const EMIT: bool>(
    cs: usize,
    board: &Board<N>,
    usable: &[Board<N>; 4],
    cands: &[Board<N>; 4],
    missing: &[Board<N>; 4],
    kicked: &[Board<N>; 4],
    kicked_hi: &[Board<N>; 4],
    remaining: u64,
    total: u64,
) -> (Moves<N>, u64) {
    if EMIT {
        let mut moves = Moves::empty(P);

        unroll!(r, cs, {
            moves.none[r] = cands[r] ^ missing[r];
        });

        if matches!(P, Piece::T) && !matches!(RULE.spins, Spins::None) && RULE.has_t_corner_spins() {
            let c0 = Board::<N>(Simd::splat(COL0));
            let c9 = Board::<N>(Simd::splat(COL9));
            let mut floor = [0u64; N];
            floor[0] = 0x3FF;
            let r0 = Board::<N>(Simd::from_array(floor));

            let ul = c0 | board.shifted(1, -1);
            let ur = c9 | board.shifted(-1, -1);
            let dl = c0 | r0 | board.shifted(1, 1);
            let dr = c9 | r0 | board.shifted(-1, 1);
            let has_3 = (ul & ur & (dl | dr)) | (dl & dr & (ul | ur));

            unroll!(r, cs, {
                let front = match r {
                    0 => ul & ur,
                    1 => ur & dr,
                    2 => dl & dr,
                    3 => ul & dl,
                    _ => unreachable!(),
                };
                let spin = (cands[r] ^ missing[r]) & kicked[r];
                moves.full[r] |= spin & has_3 & (front | kicked_hi[r]);
                moves.mini[r] |= spin & has_3 & !front & !kicked_hi[r];

                if RULE.has_immobile_t_spins() {
                    let immobile = !(usable[r].shifted(0, -1)
                        | usable[r].shifted(0, 1)
                        | usable[r].shifted(1, 0)
                        | usable[r].shifted(-1, 0));
                    moves.mini[r] |= spin & !has_3 & immobile;
                }
            });
        }

        if !matches!(P, Piece::T) && !matches!(P, Piece::O) && RULE.has_immobile_non_t_spins() {
            unroll!(r, cs, {
                let immobile = !(usable[r].shifted(0, -1)
                    | usable[r].shifted(0, 1)
                    | usable[r].shifted(1, 0)
                    | usable[r].shifted(-1, 0));
                let spin = (cands[r] ^ missing[r]) & kicked[r] & immobile;
                if RULE.is_full() {
                    moves.full[r] |= spin;
                } else {
                    moves.mini[r] |= spin;
                }
            });
        }

        if matches!(RULE.spins, Spins::Stupid) {
            unroll!(r, cs, {
                moves.full[r] |= (cands[r] ^ missing[r]) & kicked[r];
            });
        }

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
    kicked: &mut [Board<N>; 4],
    kicked_hi: &mut [Board<N>; 4],
    track: bool,
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

        if *remaining == 0 && !track {
            *done = all_done;
        } else {
            if !matches!(P, Piece::O) {
                let probe = env_probe(&search[R], EnvelopeTable::<P, R>::E);

                // rotation directions 0 and 1 (cw and ccw); 6 kicks (index 0-5)
                rot_kick_seq::<N, P, RULE, R, 0>(probe, search, unsearched, missing, done, remaining, usable, kicked, kicked_hi, track);
                rot_kick_seq::<N, P, RULE, R, 1>(probe, search, unsearched, missing, done, remaining, usable, kicked, kicked_hi, track);

                // rotation direction 2 (180)
                if const { RULE.allow_180 } {
                    rot_kick_seq::<N, P, RULE, R, 2>(probe, search, unsearched, missing, done, remaining, usable, kicked, kicked_hi, track);
                }

                if *remaining == 0 {
                    *done = all_done;
                }
            } else if *remaining == 0 {
                *done = all_done;
            }

            if *done != all_done {
                search[R] = Board::empty();
            }
        }
    }
}

#[inline]
#[allow(unused_assignments)]
fn rot_kick_seq<const N: usize, const P: Piece, const RULE: Rule, const R: usize, const D: usize>(
    probe: Board<N>,
    search: &mut [Board<N>; 4],
    unsearched: &mut [Board<N>; 4],
    missing: &mut [Board<N>; 4],
    done: &mut u64,
    remaining: &mut u64,
    usable: &[Board<N>; 4],
    kicked: &mut [Board<N>; 4],
    kicked_hi: &mut [Board<N>; 4],
    track: bool,
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
    let mut tkick = track.then_some(search[R]);

    macro_rules! step {
        ($n:literal) => {{
            let lane = kt[R][r1];
            if $n >= lane.1 {
                return;
            }

            let kx = i32::from(lane.0[$n].0) + off_x;
            let ky = i32::from(lane.0[$n].1) + off_y;

            let cand = temp.shifted(kx, ky);

            if track {
                let t = tkick.unwrap();
                let kres = t.shifted(kx, ky) & usable[r1c];
                kicked[r1c] |= kres;
                if $n >= 4 {
                    kicked_hi[r1c] |= kres;
                }
                if $n < 5 {
                    tkick = Some(t & !usable[r1c].shifted(-kx, -ky));
                }
            }

            let res = cand & unsearched[r1];

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
