//! Shared geometry constants and compile-time lookup tables.
use crate::piece::Piece;

/// Board width in cells.
pub const WIDTH: i32 = 10;
/// Number of logical rows packed into one 64-bit board word.
pub const TLINES: i32 = 6;
/// Bit mask covering the valid 60 bits in a packed 10x6 word.
pub const TALL: u64 = (1u64 << 60) - 1;
/// Spawn x-coordinate used by higher-level placement logic.
pub const SPAWN_X: i32 = 4;
/// Spawn y-coordinate used by higher-level placement logic.
pub const SPAWN_Y: i32 = 19;

#[must_use]
/// Builds a column mask for one x-position across a packed 10x6 word.
pub const fn col_word(x: i32) -> u64 {
    let mut w = 0u64;
    let mut k = 0;
    while k < TLINES {
        w |= 1u64 << (k * WIDTH + x);
        k += 1;
    }
    w
}

/// Left wall column mask in a packed word.
pub const COL0: u64 = col_word(0);
/// Right wall column mask in a packed word.
pub const COL9: u64 = col_word(9);

#[must_use]
/// Builds a mask containing columns `[0, n)` in a packed word.
pub const fn cols_below(n: i32) -> u64 {
    let mut w = 0u64;
    let mut c = 0;
    while c < n {
        w |= col_word(c);
        c += 1;
    }
    w
}

#[inline]
#[must_use]
/// Returns the horizontal shift validity mask for a shift of `dx` columns.
pub const fn dx_mask(dx: i32) -> u64 {
    if dx > 0 {
        TALL & !cols_below(dx)
    } else if dx < 0 {
        TALL & !(cols_below(-dx) << ((WIDTH + dx) as u32))
    } else {
        TALL
    }
}

#[must_use]
/// Rotates an `(x, y)` cell offset by a canonical quarter-turn index.
pub const fn rot_cell(c: (i8, i8), r: usize) -> (i8, i8) {
    match r {
        0 => c,
        1 => (c.1, -c.0),
        2 => (-c.0, -c.1),
        _ => (-c.1, c.0),
    }
}

/// Precomputes the three non-origin mino offsets for each piece and rotation.
#[must_use]
pub const fn build_pcells() -> [[[(i8, i8); 3]; 4]; 7] {
    let mut out = [[[(0i8, 0i8); 3]; 4]; 7];
    let mut p = 0;
    while p < 7 {
        let base = Piece::from_u8(p as u8).unwrap().base_cells();
        let mut r = 0;
        while r < 4 {
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
}

/// Canonical rotated cell offsets for each piece and rotation.
pub const PCELLS: [[[(i8, i8); 3]; 4]; 7] = build_pcells();

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
        while rc < Piece::from_u8(p as u8).unwrap().canonical_rotations() {
            let cells = [
                (0i8, 0i8),
                PCELLS[p][rc][0],
                PCELLS[p][rc][1],
                PCELLS[p][rc][2],
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
pub static PMASK: PlaceMaskTable = build_place_masks();

#[must_use]
/// Returns topmost occupied offset + 1 for a piece in rotation `rc`.
pub const fn top_extent(p: Piece, rc: usize) -> i32 {
    let cells = PCELLS[p as usize][rc];
    let mut top = 0i32;
    let mut i = 0;
    while i < 3 {
        if cells[i].1 as i32 > top {
            top = cells[i].1 as i32;
        }
        i += 1;
    }
    top + 1
}
