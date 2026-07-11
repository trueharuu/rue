//! Reachability search for piece placements across translations and SRS kicks.
//!
//! A note on spin detection:
//!
//! Spin policies are ordered. Any valid spin under [`Spins::T`] is also valid under [`Spins::AllMini`] and [`Spins::AllPlus`].
//! Any valid spin under [`Spins::AllMini`] is also valid under [`Spins::AllPlus`].
//! The only difference between [`Spins::AllMini`] and [`Spins::AllPlus`] is that the latter emits immobile placements as full spins, while the former emits them as mini spins.
//!
//! On any ruleset that meets [`Spins::has_3corner`], these conditions emit a spin placement:
//! - The piece is a [`Piece::T`]
//! - The placement was reached via rotation.
//! - At least 3 of 4 corners around the center are occupied. Out-of-bounds corners are always occupied.
//! - If two "front" corners are occupied, the placement is a [`Spin::Full`]. Otherwise, it is a [`Spin::Mini`].
//! - However, if this placement was reached via the 5th SRS kick, it is always a [`Spin::Full`].
//! - Any sitation where the placement can be reached with both rotation and by translation should emit for both spin types.
//!
//! On any ruleset that meets [`Spins::has_immobile`], these conditions emit a spin placement:
//! - The piece is anything except [`Piece::O`].
//! - The placement is immobile, meaning it cannot be moved in any direction (up, down, left, right).
//! - On [`Spins::AllMini`], this emits a [`Spin::Mini`] placement.
//! - On [`Spins::AllPlus`], this emits a [`Spin::Full`],
//!   except for cases where a [`Spin::Mini`] was already emitted for the same placement via the 3-corner rule.
//!   In that case, the immobile placement is ignored.

use crate::buffer::Moves;
use crate::collision::{landable_map, usable_map};
use crate::movegen::op::{horizontal_tuck, kick_step, vertical_ceiling};
use crate::unroll;
use rue_core::{
    board::Board,
    data::KickTab,
    envelope::{EnvelopeTable, env_probe},
    header::{SPAWN_X, SPAWN_Y, TLINES},
    piece::Piece,
    spin::Spins,
};

#[must_use]
/// Generates reachable/landable placements for piece `P` on board `b`.
///
/// When `EMIT` is `true`, returns populated move buckets and a zero count.
/// When `EMIT` is `false`, returns empty buckets and a reachable placement count.
pub fn gen_impl<const P: Piece, const SPINS: Spins, const N: usize, const EMIT: bool>(
    b: &Board<N>,
    y: i32,
    force: i32,
) -> (Moves<N>, u32) {
    let h: i32 = TLINES * N as i32;
    let cs = P.canonical_rotations();
    let ss = P.search_size();
    let all_done: u32 = (1u32 << ss) - 1;

    let usable = usable_map::<P, N>(b);

    let mut missing = [Board::<N>::EMPTY; 4];
    let mut search = [Board::<N>::EMPTY; 4];
    let mut via_rotation = [Board::<N>::EMPTY; 4];
    let mut via_5th_kick = [Board::<N>::EMPTY; 4];

    let mut remaining: u32 = 0;
    let mut done: u32;
    let mut total: u32 = 0;

    macro_rules! finish {
        () => {{
            if EMIT {
                let cands = landable_map(&usable, cs);
                let mut moves = Moves::empty(P);

                unroll!(r, cs, {
                    let landable = cands[r] & !missing[r];

                    let immobile = if SPINS.has_immobile() {
                        landable
                            & !usable[r].shifted(0, -1)
                            & !usable[r].shifted(0, 1)
                            & !usable[r].shifted(-1, 0)
                            & !usable[r].shifted(1, 0)
                    } else {
                        Board::<N>::EMPTY
                    };

                    let ul = (*b).shifted(1, -1) | Board::col_mask(0);
                    let ur = (*b).shifted(-1, -1) | Board::col_mask(9);
                    let dl = (*b).shifted(1, 1) | Board::col_mask(0);
                    let dr = (*b).shifted(-1, 1) | Board::col_mask(9);

                    let has3 = (ul & ur & dl) | (ul & ur & dr) | (ul & dl & dr) | (ur & dl & dr);

                    let front2 = match r {
                        0 => ul & ur,
                        1 => ur & dr,
                        2 => dr & dl,
                        3 => dl & ul,
                        _ => unreachable!(),
                    };

                    moves.landed[r] = landable;
                    moves.front2[r] = has3 & front2 & cands[r];
                    moves.has3[r] = has3 & cands[r];
                    moves.candidates[r] = usable[r];
                    moves.via_5th_kick[r] = via_5th_kick[r];
                    moves.via_rotation[r] = via_rotation[r];

                    let is_t = if matches!(P, Piece::T) {
                        !Board::<N>::EMPTY
                    } else {
                        Board::<N>::EMPTY
                    };
                    let is_t_full = is_t & has3 & (front2 | via_5th_kick[r]) & via_rotation[r] & landable;
                    let is_t_mini = is_t & has3 & !front2 & !via_5th_kick[r] & via_rotation[r] & landable;

                    // todo: 3-corner t-spin detection
                    match SPINS {
                        Spins::None => {
                            moves.none[r] |= landable;
                        }
                        Spins::T => {
                            moves.none[r] |= landable & !is_t_full;

                            moves.mini[r] |= is_t_mini;
                            moves.full[r] |= is_t_full;
                        }
                        Spins::AllMini => {
                            moves.none[r] |= landable & !immobile;
                            moves.mini[r] |= immobile;

                            moves.none[r] |= landable & (!is_t_mini | immobile);
                            moves.mini[r] |= is_t_mini;
                            moves.full[r] |= is_t_full;
                        }
                        Spins::AllPlus => {
                            moves.none[r] |= landable & !immobile;
                            moves.full[r] |= immobile;

                            moves.none[r] |= landable & (!is_t_mini | immobile);
                            moves.mini[r] |= is_t_mini;
                            moves.full[r] |= is_t_full;
                        }
                    }
                });
                return (moves, 0);
            }

            let mut miss = 0u32;
            unroll!(r, 4, {
                if remaining & (1 << r) != 0 {
                    miss += missing[r].popcount();
                }
            });
            return (Moves::empty(P), total - miss);
        }};
    }

    {
        let cands = landable_map(&usable, cs);
        if !EMIT {
            unroll!(r, cs, {
                total += cands[r].popcount();
            });
        }

        if h > SPAWN_Y && y > SPAWN_Y - P.h_spawn() {
            let threshold = (SPAWN_Y + force + 1).min(h);
            let mut s = SPAWN_Y;
            while s < threshold && !usable[0].get(SPAWN_X, s) {
                s += 1;
            }
            if s == threshold {
                return (Moves::empty(P), 0);
            }
            search[0].set(SPAWN_X, s);
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
                let surface = !usable[r];
                let surface = vertical_ceiling(surface, ceiling);
                search[r] = !surface;
                missing[r] = cands[r] & !search[r];
                if missing[r].any() {
                    remaining |= 1 << r;
                }
            });
            if remaining == 0 {
                finish!();
            }

            unroll!(r, cs, {
                let mut s = search[r];
                s = horizontal_tuck(s, &usable[r]);
                s = horizontal_tuck(s, &usable[r]);
                search[r] = s;
            });

            if P.group3() {
                unroll!(r, 4, {
                    search[r] |= (search[(r + 1) & 3] | search[(r + 3) & 3]) & usable[r];
                });
            }

            remaining = 0;
            unroll!(r, cs, {
                missing[r] ^= search[r];
                if missing[r].any() {
                    remaining |= 1 << r;
                }
            });
            if remaining == 0 {
                finish!();
            }
            if P.group2() {
                search[2] = search[0];
                search[3] = search[1];
            }
            done = 0;
        }
    }

    // BFS over nominal rotations with masked first-valid-kick waves.
    let mut unsearched = [Board::<N>::EMPTY; 4];
    unroll!(rs, ss, {
        unsearched[rs] = (!search[rs]) & usable[const { P.canonical_rotation(rs) }];
    });

    macro_rules! rot_kick {
        ($r:literal, $d:literal, $kick_idx:literal, $probe:ident) => {{
            let r1 = const { KickTab::<P, $d, $r>::R1 };
            let r1c = const { KickTab::<P, $d, $r>::R1C };
            if $probe.any() {
                let mut temp = search[$r];
                let mut prior = Board::<N>::EMPTY;
                if $kick_idx >= 1 {
                    kick_step::<P, $d, $r, 0, N>(&mut temp, &mut prior, &usable[r1c]);
                }
                if $kick_idx >= 2 {
                    kick_step::<P, $d, $r, 1, N>(&mut temp, &mut prior, &usable[r1c]);
                }
                if $kick_idx >= 3 {
                    kick_step::<P, $d, $r, 2, N>(&mut temp, &mut prior, &usable[r1c]);
                }
                if $kick_idx >= 4 {
                    kick_step::<P, $d, $r, 3, N>(&mut temp, &mut prior, &usable[r1c]);
                }
                let mut result = Board::<N>::EMPTY;
                kick_step::<P, $d, $r, $kick_idx, N>(&mut temp, &mut result, &usable[r1c]);

                let res = result & unsearched[r1];
                if res.any() {
                    search[r1] |= res;
                    unsearched[r1] ^= res;
                    done ^= 1u32 << r1;
                    missing[r1c] ^= res;

                    via_rotation[r1c] |= res;
                    if $kick_idx == 4 {
                        via_5th_kick[r1c] |= res;
                    }

                    if missing[r1c].any() {
                        remaining |= 1 << r1c;
                    } else {
                        remaining ^= (1u32 << r1c);
                    }
                }
            }
        }};
    }

    macro_rules! process_rot {
        ($r:literal) => {
            if $r < ss && done & (1 << $r) == 0 {
                done |= 1 << $r;
                let rc = const { P.canonical_rotation($r) };

                loop {
                    let temp = (search[$r].shifted(-1, 0)
                        | search[$r].shifted(1, 0)
                        | search[$r].shifted(0, -1))
                        & unsearched[$r];

                    if !temp.any() {
                        break;
                    }
                    search[$r] = search[$r] | temp;
                    unsearched[$r] = unsearched[$r] ^ temp;
                }

                missing[rc] ^= search[$r];
                if missing[rc].any() {
                    remaining |= 1 << rc;
                } else {
                    remaining ^= (1u32 << rc);
                }

                if remaining == 0 {
                    done = all_done;
                } else {
                    if !matches!(P, Piece::O) {
                        let probe = env_probe(&search[$r], EnvelopeTable::<P, $r>::E);
                        // Rotation direction 0: 5 kicks (indices 0-4)
                        rot_kick!($r, 0, 0, probe);
                        rot_kick!($r, 0, 1, probe);
                        rot_kick!($r, 0, 2, probe);
                        rot_kick!($r, 0, 3, probe);
                        rot_kick!($r, 0, 4, probe);
                        // Rotation direction 1: 5 kicks (indices 0-4)
                        rot_kick!($r, 1, 0, probe);
                        rot_kick!($r, 1, 1, probe);
                        rot_kick!($r, 1, 2, probe);
                        rot_kick!($r, 1, 3, probe);
                        rot_kick!($r, 1, 4, probe);
                        if remaining == 0 {
                            done = all_done;
                        }
                    }
                    if done != all_done {
                        search[$r] = Board::EMPTY;
                    }
                }
            }
        };
    }

    while done != all_done {
        process_rot!(0);
        process_rot!(1);
        process_rot!(2);
        process_rot!(3);
    }

    finish!();
}
