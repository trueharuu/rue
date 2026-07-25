//! SRS kick data and compile-time kick row lookup utilities.

use crate::piece::Piece;

/// Five kick offsets tested for one rotation attempt.
pub type K5 = [(i8, i8); 5];

// T.NE=(0,0)(-1,0)(-1,1)(0,-2)(-1,-2)
// T.ES=(0,0)(1,0)(1,-1)(0,2)(1,2)
// T.SW=(0,0)(1,0)(1,1)(0,-2)(1,-2)
// T.WN=(0,0)(-1,0)(-1,-1)(0,2)(-1,2)
// T.NW=(0,0)(1,0)(1,1)(0,-2)(1,-2)
// T.WS=(0,0)(-1,0)(-1,-1)(0,2)(-1,2)
// T.SE=(0,0)(-1,0)(-1,1)(0,-2)(-1,-2)
// T.EN=(0,0)(1,0)(1,-1)(0,2)(1,2)
/// Kick rows for J/L/S/T/Z pieces indexed by direction then rotation.
pub const KICKS_LJSZT: [[K5; 4]; 2] = [
    [
        [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
        [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
        [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
        [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    ],
    [
        [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
        [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
        [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
        [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    ],
];

// I.NE=(1,0)(-1,0)(2,0)(-1,-1)(2,2)
// I.ES=(0,-1)(-1,-1)(2,-1)(-1,1)(2,-2)
// I.SW=(-1,0)(1,0)(-2,0)(1,1)(-2,-2)
// I.WN=(0,1)(1,1)(-2,1)(1,-1)(-2,2)
// I.NW=(0,-1)(-1,-1)(2,-1)(-1,1)(2,-2)
// I.WS=(1,0)(-1,0)(2,0)(-1,-1)(2,2)
// I.SE=(0,1)(1,1)(-2,1)(1,-1)(-2,2)
// I.EN=(-1,0)(1,0)(-2,0)(1,1)(-2,-2)
/// Kick rows for I pieces indexed by direction then rotation.
pub const KICKS_I: [[K5; 4]; 2] = [
    [
        [(1, 0), (-1, 0), (2, 0), (-1, -1), (2, 2)],
        [(0, -1), (-1, -1), (2, -1), (-1, 1), (2, -2)],
        [(-1, 0), (1, 0), (-2, 0), (1, 1), (-2, -2)],
        [(0, 1), (1, 1), (-2, 1), (1, -1), (-2, 2)],
    ],
    [
        [(0, -1), (-1, -1), (2, -1), (-1, 1), (2, -2)],
        [(-1, 0), (1, 0), (-2, 0), (1, 1), (-2, -2)],
        [(0, 1), (1, 1), (-2, 1), (1, -1), (-2, 2)],
        [(1, 0), (-1, 0), (2, 0), (-1, -1), (2, 2)],
    ],
];

// I.NE=(1,0)(2,0)(-1,0)(-1,-1)(2,2)
// I.ES=(0,-1)(-1,-1)(2,-1)(-1,1)(2,-2)
// I.SW=(-1,0)(1,0)(-2,0)(1,1)(-2,-2)
// I.WN=(0,1)(1,1)(-2,1)(1,-1)(-2,2)
// I.NW=(0,-1)(-1,-1)(2,-1)(2,-2)(-1,1)
// I.WS=(1,0)(2,0)(-1,0)(2,2)(-1,-1)
// I.SE=(0,1)(-2,1)(1,1)(-2,2)(1,-1)
// I.EN=(-1,0)(-2,0)(1,0)(-2,-2)(1,1)
/// TETR.IO SRS+ kick rows for I pieces indexed by direction then rotation.
pub const KICKS_I_TETRIO: [[K5; 4]; 2] = [
    [
        [(1, 0), (2, 0), (-1, 0), (-1, -1), (2, 2)],
        [(0, -1), (-1, -1), (2, -1), (-1, 1), (2, -2)],
        [(-1, 0), (1, 0), (-2, 0), (1, 1), (-2, -2)],
        [(0, 1), (1, 1), (-2, 1), (1, -1), (-2, 2)],
    ],
    [
        [(0, -1), (-1, -1), (2, -1), (2, -2), (-1, 1)],
        [(-1, 0), (-2, 0), (1, 0), (-2, -2), (1, 1)],
        [(0, 1), (-2, 1), (1, 1), (-2, 2), (1, -1)],
        [(1, 0), (2, 0), (-1, 0), (2, 2), (-1, -1)],
    ],
];

/// Six kick offsets tested for a 180-degree rotation attempt.
type K6 = [(i8, i8); 6];

// T.NS=(0,0)(0,1)(1,1)(-1,1)(1,0)(-1,0)
// T.EW=(0,0)(1,0)(1,2)(1,1)(0,2)(0,1)
// T.SN=(0,0)(0,-1)(-1,-1)(1,-1)(-1,0)(1,0)
// T.WE=(0,0)(-1,0)(-1,2)(-1,1)(0,2)(0,1)
/// TETR.IO SRS+ 180-degree kick rows for J/L/S/T/Z pieces indexed by source rotation.
pub const KICKS_LJSZT_180: [K6; 4] = [
    [(0, 0), (0, 1), (1, 1), (-1, 1), (1, 0), (-1, 0)],
    [(0, 0), (1, 0), (1, 2), (1, 1), (0, 2), (0, 1)],
    [(0, 0), (0, -1), (-1, -1), (1, -1), (-1, 0), (1, 0)],
    [(0, 0), (-1, 0), (-1, 2), (-1, 1), (0, 2), (0, 1)],
];

// T.NS=(0,0)(0,1)
// T.EW=(0,0)(1,0)
// T.SN=(0,0)(0,-1)
// T.WE=(0,0)(-1,0)
/// Standard SRS 180-degree kick rows for J/L/S/T/Z pieces indexed by source rotation.
pub const KICKS_LJSZT_180_JSTRIS: [K6; 4] = [
    [(0, 0), (0, 1), (0, 0), (0, 0), (0, 0), (0, 0)],
    [(0, 0), (1, 0), (0, 0), (0, 0), (0, 0), (0, 0)],
    [(0, 0), (0, -1), (0, 0), (0, 0), (0, 0), (0, 0)],
    [(0, 0), (-1, 0), (0, 0), (0, 0), (0, 0), (0, 0)],
];

// I.NS=(1,-1)(1,0)
// I.EW=(-1,-1)(0,-1)
// I.SN=(-1,1)(-1,0)
// I.WE=(1,1)(0,1)
/// 180-degree kick rows for I pieces indexed by source rotation (same for SRS+ and standard).
pub const KICKS_I_180: [K6; 4] = [
    [(1, -1), (1, 0), (0, 0), (0, 0), (0, 0), (0, 0)],
    [(-1, -1), (0, -1), (0, 0), (0, 0), (0, 0), (0, 0)],
    [(-1, 1), (-1, 0), (0, 0), (0, 0), (0, 0), (0, 0)],
    [(1, 1), (0, 1), (0, 0), (0, 0), (0, 0), (0, 0)],
];

#[must_use]
/// Returns the kick row for piece `p`, direction `d` (`0` cw, `1` ccw), rotation `r`, and `srs_plus` flag.
pub const fn kick_row_const(p: Piece, d: usize, r: usize, srs_plus: bool) -> K5 {
    if matches!(p, Piece::I) && srs_plus {
        KICKS_I_TETRIO[d][r]
    } else if matches!(p, Piece::I) {
        KICKS_I[d][r]
    } else {
        KICKS_LJSZT[d][r]
    }
}

/// Compile-time kick table accessor specialized by piece/direction/rotation const params.
pub struct KickTab<const P: Piece, const D: usize, const R: usize>;

impl<const P: Piece, const D: usize, const R: usize> KickTab<P, D, R> {
    /// Destination rotation index for this kick transition.
    pub const R1: usize = if D == 0 { (R + 1) & 3 } else { (R + 3) & 3 };
    /// Canonicalized destination rotation for symmetry-reduced processing.
    pub const R1C: usize = P.canonical_rotation(Self::R1);
    /// Canonical frame x-offset between source and destination rotations.
    pub const OFF_X: i32 = P.canonical_offset(R).0 - P.canonical_offset(Self::R1).0;
    /// Canonical frame y-offset between source and destination rotations.
    pub const OFF_Y: i32 = P.canonical_offset(R).1 - P.canonical_offset(Self::R1).1;
}
