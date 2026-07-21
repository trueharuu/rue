//! Height tracking and dynamic band-width selection.
//!
//! The perft driver selects the smallest banded board that fits the current
//! height to minimize memory usage. Band width is determined by the height
//! after placing a piece and running initial move generation.

use rue_core::header::top_extent;
use rue_core::piece::Piece;

/// Select the minimum band width (in u64 words) needed to store height `h`.
/// Each word holds 6 rows ([`TLINES`]), so we need `ceil(h / TLINES)` words.
/// Options are capped at [1, 2, 3, 4, 8].
///
/// [`TLINES`]: [`rue_core::header::TLINES`]
#[inline]
#[must_use]
pub fn band_words(h: i32) -> usize {
    if h < 6 {
        1
    } else if h < 12 {
        2
    } else if h < 18 {
        3
    } else if h < 24 {
        4
    } else {
        8
    }
}

#[inline]
#[must_use]
/// Computes the post-lock height when no line clear occurs.
///
/// This uses only lock y-coordinate and piece top extent, avoiding a full
/// board scan.
pub fn height_after_clear_free<const P: Piece>(y: i32, rc: usize, old_h: i32) -> i32 {
    let extent = top_extent(P, rc);
    let t = y + extent;
    if t > old_h { t } else { old_h }
}

#[must_use]
/// Parses a queue string like `"IOTLJSZ"` into a piece sequence.
///
/// Returns `None` when any character is not a valid tetromino symbol.
pub fn parse_queue(s: &str) -> Option<Vec<Piece>> {
    s.chars()
        .map(|c| match c {
            'T' => Some(Piece::T),
            'I' => Some(Piece::I),
            'J' => Some(Piece::J),
            'L' => Some(Piece::L),
            'O' => Some(Piece::O),
            'S' => Some(Piece::S),
            'Z' => Some(Piece::Z),
            _ => None,
        })
        .collect()
}
