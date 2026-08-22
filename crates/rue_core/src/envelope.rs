//! Rotation envelope helpers for conservative occupancy expansion.

use crate::board::Board;
use crate::data::KICKS_I;
use crate::data::KICKS_O;
use crate::data::KICKS_TJLSZ;
use crate::piece::Piece;

/// Computes the min/max kick reach `(xmin, xmax, ymin, ymax)` across cw/ccw kicks.
#[inline]
#[must_use]
pub const fn env_union(p: Piece, r: usize) -> (i32, i32, i32, i32) {
    let kt = match p {
        Piece::I => &KICKS_I,
        Piece::O => &KICKS_O,
        _ => &KICKS_TJLSZ,
    };
    let (mut xmin, mut xmax, mut ymin, mut ymax) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    let mut d = 0;
    while d < 2 {
        let r1 = if d == 0 { (r + 1) & 3 } else { (r + 3) & 3 };
        let mut i = 0;
        while i < 5 {
            let kx = kt[r][r1].0[i].0 as i32;
            let ky = kt[r][r1].0[i].1 as i32;
            if kx < xmin {
                xmin = kx;
            }
            if kx > xmax {
                xmax = kx;
            }
            if ky < ymin {
                ymin = ky;
            }
            if ky > ymax {
                ymax = ky;
            }
            i += 1;
        }
        d += 1;
    }
    (xmin, xmax, ymin, ymax)
}

/// Compile-time envelope accessor specialized by piece and rotation.
pub struct EnvelopeTable<const P: Piece, const R: usize>;

impl<const P: Piece, const R: usize> EnvelopeTable<P, R> {
    /// Envelope bounds for `(P, R)`.
    pub const E: (i32, i32, i32, i32) = env_union(P, R);
}

/// Expands occupied cells by envelope reach to produce candidate collision probes.
#[inline]
#[must_use]
pub fn env_probe<const N: usize>(s: &Board<N>, e: (i32, i32, i32, i32)) -> Board<N> {
    let (xmin, xmax, ymin, ymax) = e;
    let mut h = *s;
    if xmin <= -1 {
        h |= s.shifted(-1, 0);
    }
    if xmin <= -2 {
        h |= s.shifted(-2, 0);
    }
    if xmin <= -3 {
        h |= s.shifted(-3, 0);
    }
    if xmax >= 1 {
        h |= s.shifted(1, 0);
    }
    if xmax >= 2 {
        h |= s.shifted(2, 0);
    }
    if xmax >= 3 {
        h |= s.shifted(3, 0);
    }
    let hh = h;
    let mut v = hh;
    if ymin <= -1 {
        v |= hh.shifted(0, -1);
    }
    if ymin <= -2 {
        v |= hh.shifted(0, -2);
    }
    if ymin <= -3 {
        v |= hh.shifted(0, -3);
    }
    if ymax >= 1 {
        v |= hh.shifted(0, 1);
    }
    if ymax >= 2 {
        v |= hh.shifted(0, 2);
    }
    if ymax >= 3 {
        v |= hh.shifted(0, 3);
    }
    v
}
