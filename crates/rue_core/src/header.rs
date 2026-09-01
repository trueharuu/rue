//! Header constants for the entire project.
//! These constants are used in multiple places.
//! They are defined in one location to avoid duplication.

use crate::data::CELLS;
use crate::piece::Piece;

/// [`crate::board::Board`] width.
pub const WIDTH: i32 = 10;
/// Total number of lines per band in a [`crate::board::Board`].
pub const TLINES: i32 = 6;
/// Bit mask covering the valid 60 bits of a band word.
pub const TALL: u64 = (1u64 << (TLINES * WIDTH)) - 1;

/// Returns topmost occupied offset + 1 for a piece in rotation `rc`.
#[inline]
#[must_use]
pub const fn top_extent(p: Piece, rc: usize) -> i32 {
    let cells = CELLS[p as usize][rc];
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

#[inline]
#[must_use]
pub const fn dx_mask(dx: i32) -> u64 {
    if dx > 0 {
        TALL & !cols_below(dx)
    } else if dx < 0 {
        TALL & !(cols_below(-dx) << ((WIDTH + dx) as u32))
    } else {
        TALL
    }
}

#[inline]
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

#[inline]
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
