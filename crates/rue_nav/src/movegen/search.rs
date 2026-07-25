//! Reachability search for piece placements across translations and SRS kicks.

use crate::buffer::Moves;
use crate::collision::landable_map;
use crate::collision::usable_map;
use crate::movegen::op::horizontal_tuck;
use crate::movegen::op::kick_step;
use crate::movegen::op::vertical_ceiling;
use crate::unroll;
use rue_core::board::Board;
use rue_core::data::KickTab;
use rue_core::data::kick_row_const;
use rue_core::envelope::EnvelopeTable;
use rue_core::envelope::env_probe;
use rue_core::game::ruleset::Handling;
use rue_core::header::SPAWN_X;
use rue_core::header::SPAWN_Y;
use rue_core::header::TLINES;
use rue_core::header::WIDTH;
use rue_core::piece::Piece;
use rue_core::spin::Spins;

#[allow(clippy::similar_names)]
#[must_use]
/// Generates reachable/landable placements for piece `P` on board `b`.
///
/// When `EMIT` is `true`, returns populated move buckets and a zero count.
/// When `EMIT` is `false`, returns empty buckets and a reachable placement count.
pub fn gen_impl<const P: Piece, const RULE: Handling, const N: usize, const EMIT: bool>(
    b: &Board<N>,
    y: i32,
    force: i32,
) -> (Moves<N>, u32) {
    let h: i32 = TLINES * N as i32;
    let cs = P.canonical_rotations();
    let ss = P.search_size();
    let all_done: u32 = (1u32 << ss) - 1;

    let usable = usable_map::<P, N>(b);

    // Pre-compute T-spin corner masks (Cobra's `spins` and `spinMap`).
    let (has3, front2_arr): (Board<N>, [Board<N>; 4]) = if const { P as u8 == 0 && RULE.spins as u8 != 0 }
    {
        let wall_left = Board::<N>::col_mask(0);
        let wall_right = Board::<N>::col_mask(9);
        let mut wall_bottom = Board::<N>::EMPTY;
        let mut wall_top = Board::<N>::EMPTY;
        {
            let mut x = 0i32;
            while x < WIDTH {
                wall_bottom.set(x, 0);
                wall_top.set(x, Board::<N>::H - 1);
                x += 1;
            }
        }
        let corner_tl = b.shifted(1, 1) | wall_left | wall_bottom;
        let corner_tr = b.shifted(-1, 1) | wall_right | wall_bottom;
        let corner_bl = b.shifted(1, -1) | wall_left | wall_top;
        let corner_br = b.shifted(-1, -1) | wall_right | wall_top;
        let has3 = (corner_tl & corner_tr & corner_bl)
            | (corner_tl & corner_tr & corner_br)
            | (corner_tl & corner_bl & corner_br)
            | (corner_tr & corner_bl & corner_br);
        let front2_arr = [
            corner_bl & corner_br,
            corner_tr & corner_br,
            corner_tl & corner_tr,
            corner_tl & corner_bl,
        ];
        (has3, front2_arr)
    } else {
        (Board::<N>::EMPTY, [Board::<N>::EMPTY; 4])
    };

    let mut missing = [Board::<N>::EMPTY; 4];
    let mut search = [Board::<N>::EMPTY; 4];
    let mut reached_via_rotation = [Board::<N>::EMPTY; 4];
    let mut reached_via_5th_kick = [Board::<N>::EMPTY; 4];

    let mut reached_by_translation = [Board::<N>::EMPTY; 4];

    // Cobra's `spinReach[NONE | MINI | FULL]` — accumulated during BFS.
    let mut spin_reach_none = [Board::<N>::EMPTY; 4];
    let mut spin_reach_mini = [Board::<N>::EMPTY; 4];
    let mut spin_reach_full = [Board::<N>::EMPTY; 4];

    let mut remaining: u32 = 0;
    let mut done: u32;
    let mut total: u32 = 0;

    macro_rules! finish {
        () => {{
            if EMIT {
                let cands = landable_map(&usable, cs);
                let mut moves = Moves::empty(P);

                moves.landed = cands;

                unroll!(r, cs, {
                    let landable = cands[r] & !missing[r];
                    let immobile = landable
                        & !usable[r].shifted(0, -1)
                        & !usable[r].shifted(0, 1)
                        & !usable[r].shifted(-1, 0)
                        & !usable[r].shifted(1, 0);
                    moves.immobile[r] = immobile;

                    let mut via_rot = Board::<N>::EMPTY;
                    let mut via_5th = Board::<N>::EMPTY;
                    unroll!(srs, 4, {
                        if const { P.canonical_rotation(srs) } == r {
                            via_rot |= reached_via_rotation[srs];
                            via_5th |= reached_via_5th_kick[srs];
                        }
                    });
                    moves.via_rotation[r] = via_rot & landable;
                    moves.via_5th_kick[r] = via_5th & landable;

                    moves.has3[r] = has3 & landable;
                    moves.front2[r] = front2_arr[r] & landable;

                    if const { P as u8 == 0 } {
                        match RULE.spins {
                            Spins::None => {
                                moves.none[r] = landable;
                            }
                            Spins::T | Spins::AllMini | Spins::AllPlus => {
                                // Cobra: moves[s][rs] = candidates[rs] & spinReach[s][rs]
                                moves.full[r] = spin_reach_full[r] & landable;
                                let mini_immobile = if const { RULE.spins as u8 == 1 } {
                                    Board::<N>::EMPTY
                                } else {
                                    via_rot & immobile & !(has3 & landable)
                                };
                                moves.mini[r] = (spin_reach_mini[r] & landable) | mini_immobile;
                                moves.none[r] = spin_reach_none[r]
                                    & !spin_reach_mini[r]
                                    & !spin_reach_full[r]
                                    & landable;
                            }
                        }
                    } else {
                        match RULE.spins {
                            Spins::None | Spins::T => {
                                moves.none[r] = landable;
                            }
                            Spins::AllMini => {
                                moves.none[r] = reached_by_translation[r] & landable;
                                moves.mini[r] = immobile;
                            }
                            Spins::AllPlus => {
                                moves.none[r] = reached_by_translation[r] & landable;
                                moves.full[r] = immobile;
                            }
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
                unroll!(r, cs, {
                    reached_by_translation[r] = search[r];
                    spin_reach_none[r] = search[r];
                });
                finish!();
            }

            // Two rounds of horizontal tucks (pure translation, no rotation)
            unroll!(r, cs, {
                let mut s = search[r];
                s = horizontal_tuck(s, &usable[r]);
                s = horizontal_tuck(s, &usable[r]);
                search[r] = s;
            });

            if P.group3() {
                // Propagate seeds between group-3 rotations (pure translation)
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
                unroll!(r, cs, {
                    reached_by_translation[r] = search[r];
                    spin_reach_none[r] = search[r];
                });
                finish!();
            }
            if P.group2() {
                search[2] = search[0];
                search[3] = search[1];
            }
            done = 0;
        }
    }

    // Save translation-reachable positions before BFS adds rotation-kick paths.
    unroll!(r, cs, {
        reached_by_translation[r] = search[r];
        spin_reach_none[r] = search[r];
    });

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
                let off_x = KickTab::<P, $d, $r>::OFF_X;
                let off_y = KickTab::<P, $d, $r>::OFF_Y;
                let kick_row = kick_row_const(P, $d, $r, RULE.srs_plus);
                let mut temp = search[$r];
                let mut prior = Board::<N>::EMPTY;
                if $kick_idx >= 1 {
                    kick_step::<0, N>(&mut temp, &mut prior, &usable[r1c], &kick_row, off_x, off_y);
                }
                if $kick_idx >= 2 {
                    kick_step::<1, N>(&mut temp, &mut prior, &usable[r1c], &kick_row, off_x, off_y);
                }
                if $kick_idx >= 3 {
                    kick_step::<2, N>(&mut temp, &mut prior, &usable[r1c], &kick_row, off_x, off_y);
                }
                if $kick_idx >= 4 {
                    kick_step::<3, N>(&mut temp, &mut prior, &usable[r1c], &kick_row, off_x, off_y);
                }
                let mut result = Board::<N>::EMPTY;
                kick_step::<$kick_idx, N>(
                    &mut temp,
                    &mut result,
                    &usable[r1c],
                    &kick_row,
                    off_x,
                    off_y,
                );

                // Cobra spin reach: classify kick result into NONE / MINI / FULL.
                if const { P as u8 == 0 && RULE.spins as u8 != 0 } {
                    let spun = result & has3;
                    spin_reach_none[r1] |= result & !has3;
                    if const { $kick_idx >= 4 } {
                        spin_reach_full[r1] |= spun;
                    } else {
                        spin_reach_mini[r1] |= spun & !front2_arr[r1];
                        spin_reach_full[r1] |= spun & front2_arr[r1];
                    }
                }

                let res = result & unsearched[r1];
                reached_via_rotation[r1] |= result & usable[r1c];
                if const { $kick_idx == 4 } {
                    reached_via_5th_kick[r1] |= result & usable[r1c];
                }
                if res.any() {
                    search[r1] |= res;
                    unsearched[r1] &= !res;
                    done &= !(1u32 << r1);
                    missing[r1c] &= !res;

                    if missing[r1c].any() {
                        remaining |= 1 << r1c;
                    } else {
                        remaining &= !(1u32 << r1c);
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
                    let temp_all = search[$r].shifted(-1, 0)
                        | search[$r].shifted(1, 0)
                        | search[$r].shifted(0, -1);
                    // Cobra: spinReach[NONE][r] |= temp (before unsearched filter)
                    spin_reach_none[$r] |= temp_all;
                    let temp = temp_all & unsearched[$r];

                    if !temp.any() {
                        break;
                    }
                    search[$r] = search[$r] | temp;
                    unsearched[$r] = unsearched[$r] ^ temp;
                }

                missing[rc] &= !search[$r];
                if missing[rc].any() {
                    remaining |= 1 << rc;
                } else {
                    remaining &= !(1u32 << rc);
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

    // Expand translation reachability by gravity (same as process_rot but using usable).
    unroll!(r, cs, {
        loop {
            let new = (reached_by_translation[r].shifted(-1, 0)
                | reached_by_translation[r].shifted(1, 0)
                | reached_by_translation[r].shifted(0, -1))
                & usable[r]
                & !reached_by_translation[r];
            if !new.any() {
                break;
            }
            reached_by_translation[r] |= new;
        }
    });

    finish!();
}
