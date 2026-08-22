//! Raw data tables, containing both placement cells and rotation kick tables.

use crate::header::TLINES;
use crate::header::WIDTH;
use crate::piece::Piece;
use crate::rotation::Rotation;

/// Rotates an `(x, y)` cell offset by a canonical quarter-turn index.
#[inline]
#[must_use]
pub const fn rot_cell(c: (i8, i8), rot: usize) -> (i8, i8) {
    match rot & 3 {
        0 => c,
        1 => (c.1, -c.0),
        2 => (-c.0, -c.1),
        _ => (-c.1, c.0),
    }
}

/// Canonical rotated cell offsets for each [`Piece`] and [`Rotation`].
pub const CELLS: [[[(i8, i8); 3]; Rotation::NB]; Piece::NB] = const {
    let mut out = [[[(0, 0); _]; _]; _];
    let mut p = 0;

    while p < Piece::NB {
        let base = Piece::from_u8(p as u8).unwrap().base_cells();

        let mut r = 0;
        while r < Rotation::NB {
            let mut i = 0;
            while i < 3 {
                out[p][r][i] = rot_cell(base[i], r);
                i += 1;
            }

            r += 1;
        }

        p += 1;
    }

    out
};

/// Compact place-mask tuple `(low_word, high_word, band_offset, x_bias)`.
pub type PlaceMask = (u64, u64, i8, i8);
/// Full table of precomputed place masks indexed by piece, rotation, and y-mod.
pub type PlaceMaskTable = [[[PlaceMask; 6]; 4]; 7];

/// Builds compact placement masks used by masked board placement.
#[must_use]
pub const fn build_place_masks() -> PlaceMaskTable {
    let mut out = [[[(0u64, 0u64, 0i8, 0i8); 6]; 4]; 7];
    let mut p = 0;
    while p < 7 {
        let mut rc = 0;
        while rc < Piece::from_u8(p as u8).unwrap().groups() {
            let cells = [
                (0i8, 0i8),
                CELLS[p][rc][0],
                CELLS[p][rc][1],
                CELLS[p][rc][2],
            ];
            let mut dx = 0i32;
            let mut dy = 0i32;
            let mut i = 0;
            while i < 4 {
                if (cells[i].0 as i32) < dx {
                    dx = cells[i].0 as i32;
                }
                if (cells[i].1 as i32) < dy {
                    dy = cells[i].1 as i32;
                }
                i += 1;
            }

            let xb = -dx;
            let mut yr = 0;
            while yr < 6 {
                let boff: i32 = if yr as i32 + dy < 0 { -1 } else { 0 };
                let mut lo = 0u64;
                let mut hi = 0u64;
                let mut i = 0;
                while i < 4 {
                    let rr = yr as i32 + cells[i].1 as i32 - TLINES * boff;
                    let c = cells[i].0 as i32 + xb;
                    let bit = 1u64 << (((rr % TLINES) * WIDTH + c) as u32);
                    if rr < TLINES {
                        lo |= bit;
                    } else {
                        hi |= bit;
                    }
                    i += 1;
                }
                out[p][rc][yr] = (lo, hi, boff as i8, xb as i8);
                yr += 1;
            }
            rc += 1;
        }
        p += 1;
    }
    out
}

/// Precomputed placement mask table.
pub const PMASK: PlaceMaskTable = const {
    let mut out = [[[(0u64, 0u64, 0i8, 0i8); 6]; 4]; 7];
    let mut p = 0;
    while p < 7 {
        let mut rc = 0;
        while rc < Piece::from_u8(p as u8).unwrap().groups() {
            let cells = [
                (0i8, 0i8),
                CELLS[p][rc][0],
                CELLS[p][rc][1],
                CELLS[p][rc][2],
            ];
            let mut dx = 0i32;
            let mut dy = 0i32;
            let mut i = 0;
            while i < 4 {
                if (cells[i].0 as i32) < dx {
                    dx = cells[i].0 as i32;
                }
                if (cells[i].1 as i32) < dy {
                    dy = cells[i].1 as i32;
                }
                i += 1;
            }

            let xb = -dx;
            let mut yr = 0;
            while yr < 6 {
                let boff: i32 = if yr as i32 + dy < 0 { -1 } else { 0 };
                let mut lo = 0u64;
                let mut hi = 0u64;
                let mut i = 0;
                while i < 4 {
                    let rr = yr as i32 + cells[i].1 as i32 - TLINES * boff;
                    let c = cells[i].0 as i32 + xb;
                    let bit = 1u64 << (((rr % TLINES) * WIDTH + c) as u32);
                    if rr < TLINES {
                        lo |= bit;
                    } else {
                        hi |= bit;
                    }
                    i += 1;
                }
                out[p][rc][yr] = (lo, hi, boff as i8, xb as i8);
                yr += 1;
            }
            rc += 1;
        }
        p += 1;
    }
    out
};

/// A single kick wave for a single rotation transition, containing up to 6
/// `(dx, dy)` offsets.
pub type K6 = ([(i8, i8); 6], usize);
/// Canonical rotation kick tables for each initial and final [`Rotation`].
pub type Kicks = [[K6; Rotation::NB]; Rotation::NB];

/// A null kick wave, containing only `(0, 0)` offsets.
pub const NULL_KICK: K6 = ([(0, 0); 6], 0);

macro_rules! kick {
    ($k:expr, $i:expr, $j:expr, $(($x:expr, $y:expr))*) => {{
        #![allow(unused)]
        use Rotation::East as E;
        use Rotation::North as N;
        use Rotation::South as S;
        use Rotation::West as W;

        let mut kick = NULL_KICK;
        let mut k = 0;
        $(
            kick.0[k] = ($x, $y);
            k += 1;
        )*
        kick.1 = k;
        $k[$i as usize][$j as usize] = kick;
    }};
}

// T.NE=(0,0)(-1,0)(-1,1)(0,-2)(-1,-2)
// T.ES=(0,0)(1,0)(1,-1)(0,2)(1,2)
// T.SW=(0,0)(1,0)(1,1)(0,-2)(1,-2)
// T.WN=(0,0)(-1,0)(-1,-1)(0,2)(-1,2)
// T.NW=(0,0)(1,0)(1,1)(0,-2)(1,-2)
// T.WS=(0,0)(-1,0)(-1,-1)(0,2)(-1,2)
// T.SE=(0,0)(-1,0)(-1,1)(0,-2)(-1,-2)
// T.EN=(0,0)(1,0)(1,-1)(0,2)(1,2)
// T.NS=(0,0)(0,1)(1,1)(-1,1)(1,0)(-1,0)
// T.EW=(0,0)(1,0)(1,2)(1,1)(0,2)(0,1)
// T.SN=(0,0)(0,-1)(-1,-1)(1,-1)(-1,0)(1,0)
// T.WE=(0,0)(-1,0)(-1,2)(-1,1)(0,2)(0,1)

/// Kick table for T, J, L, S, and Z pieces.
pub const KICKS_TJLSZ: Kicks = {
    let mut out = [[NULL_KICK; Rotation::NB]; Rotation::NB];

    kick!(out, N, E, (0, 0)(-1, 0)(-1, 1)(0, -2)(-1, -2));
    kick!(out, E, S, (0, 0)(1, 0)(1, -1)(0, 2)(1, 2));
    kick!(out, S, W, (0, 0)(1, 0)(1, 1)(0, -2)(1, -2));
    kick!(out, W, N, (0, 0)(-1, 0)(-1, -1)(0, 2)(-1, 2));
    kick!(out, N, W, (0, 0)(1, 0)(1, 1)(0, -2)(1, -2));
    kick!(out, W, S, (0, 0)(-1, 0)(-1, -1)(0, 2)(-1, 2));
    kick!(out, S, E, (0, 0)(-1, 0)(-1, 1)(0, -2)(-1, -2));
    kick!(out, E, N, (0, 0)(1, 0)(1, -1)(0, 2)(1, 2));
    kick!(out, N, S, (0, 0)(0, 1)(1, 1)(-1, 1)(1, 0)(-1, 0));
    kick!(out, E, W, (0, 0)(1, 0)(1, 2)(1, 1)(0, 2)(0, 1));
    kick!(out, S, N, (0, 0)(0, -1)(-1, -1)(1, -1)(-1, 0)(1, 0));
    kick!(out, W, E, (0, 0)(-1, 0)(-1, 2)(-1, 1)(0, 2)(0, 1));

    out
};

// I.NE=(1,0)(2,0)(-1,0)(-1,-1)(2,2)
// I.ES=(0,-1)(-1,-1)(2,-1)(-1,1)(2,-2)
// I.SW=(-1,0)(1,0)(-2,0)(1,1)(-2,-2)
// I.WN=(0,1)(1,1)(-2,1)(1,-1)(-2,2)
// I.NW=(0,-1)(-1,-1)(2,-1)(2,-2)(-1,1)
// I.WS=(1,0)(2,0)(-1,0)(2,2)(-1,-1)
// I.SE=(0,1)(-2,1)(1,1)(-2,2)(1,-1)
// I.EN=(-1,0)(-2,0)(1,0)(-2,-2)(1,1)
// I.NS=(1,-1)(1,0)
// I.EW=(-1,-1)(0,-1)
// I.SN=(-1,1)(-1,0)
// I.WE=(1,1)(0,1)

/// Kick table for I pieces.
pub const KICKS_I: Kicks = {
    let mut out = [[NULL_KICK; Rotation::NB]; Rotation::NB];

    kick!(out, N, E, (1, 0)(2, 0)(-1, 0)(-1, -1)(2, 2));
    kick!(out, E, S, (0, -1)(-1, -1)(2, -1)(-1, 1)(2, -2));
    kick!(out, S, W, (-1, 0)(1, 0)(-2, 0)(1, 1)(-2, -2));
    kick!(out, W, N, (0, 1)(1, 1)(-2, 1)(1, -1)(-2, 2));
    kick!(out, N, W, (0, -1)(-1, -1)(2, -1)(2, -2)(-1, 1));
    kick!(out, W, S, (1, 0)(2, 0)(-1, 0)(2, 2)(-1, -1));
    kick!(out, S, E, (0, 1)(-2, 1)(1, 1)(-2, 2)(1, -1));
    kick!(out, E, N, (-1, 0)(-2, 0)(1, 0)(-2, -2)(1, 1));
    kick!(out, N, S, (1, -1)(1, 0));
    kick!(out, E, W, (-1, -1)(0, -1));
    kick!(out, S, N, (-1, 1)(-1, 0));
    kick!(out, W, E, (1, 1)(0, 1));

    out
};

// O.NE=(0,1)
// O.ES=(1,0)
// O.SW=(0,-1)
// O.WN=(-1,0)
// O.NW=(1,0)
// O.WS=(0,1)
// O.SE=(-1,0)
// O.EN=(0,-1)
// O.NS=(1,1)
// O.EW=(1,-1)
// O.SN=(-1,-1)
// O.WE=(-1,1)

/// Kick table for O pieces. This should not be reached.
pub const KICKS_O: Kicks = {
    let mut out = [[NULL_KICK; Rotation::NB]; Rotation::NB];

    kick!(out, N, E, (0, 1));
    kick!(out, E, S, (1, 0));
    kick!(out, S, W, (0, -1));
    kick!(out, W, N, (-1, 0));
    kick!(out, N, W, (1, 0));
    kick!(out, W, S, (0, 1));
    kick!(out, S, E, (-1, 0));
    kick!(out, E, N, (0, -1));
    kick!(out, N, S, (1, 1));
    kick!(out, E, W, (1, -1));
    kick!(out, S, N, (-1, -1));
    kick!(out, W, E, (-1, 1));

    out
};
