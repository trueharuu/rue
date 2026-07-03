//! Spin detection maps for T pieces.
use rue_core::{board::Board, piece::Piece};

/// Spin detection corner and immobility maps.
pub struct SpinMap<const N: usize, const P: Piece> {
    /// Map which holds positions where at least three corners around the center are occupied.
    pub corners: [Board<N>; 4],
    /// Subset of `corners` indicating where the front two corners are occupied.
    pub front_corners: [Board<N>; 4],
    /// Set of placements where the piece is immobile.
    pub immobile: Board<N>,
}

/// Corner offsets for Z pieces.
pub const CORNER_TABLE_Z: [[(i32, i32); 4]; 4] = [
    [(-2, -1), (1, -1), (2, 0), (-1, 0)],
    [(0, -1), (1, -2), (0, 2), (1, 1)],
    [(-2, 0), (1, 0), (2, 1), (-1, 1)],
    [(-1, -1), (0, -2), (0, 1), (-1, 2)],
];

/// Corner offsets for L pieces.
pub const CORNER_TABLE_L: [[(i32, i32); 4]; 4] = [
    [(-1, -1), (0, -1), (1, 1), (-1, 1)],
    [(-1, -1), (1, -1), (1, 0), (-1, 1)],
    [(-1, -1), (1, -1), (1, 1), (0, 1)],
    [(-1, 0), (1, -1), (1, 1), (-1, 1)],
];

/// Corner offsets for S pieces.
pub const CORNER_TABLE_S: [[(i32, i32); 4]; 4] = [
    [(-1, -1), (2, -1), (1, 0), (-2, 0)],
    [(0, -2), (1, -1), (1, 2), (0, 1)],
    [(-1, 0), (2, 0), (1, 1), (-2, 1)],
    [(-1, -2), (0, -1), (-1, 1), (0, 2)],
];

/// Corner offsets for J pieces.
pub const CORNER_TABLE_J: [[(i32, i32); 4]; 4] = [
    [(0, -1), (1, -1), (1, 1), (-1, 1)],
    [(-1, -1), (1, 0), (1, 1), (-1, 1)],
    [(-1, -1), (1, -1), (0, 1), (-1, 1)],
    [(-1, -1), (1, -1), (1, 1), (-1, 0)],
];

/// Corner offsets for T pieces.
pub const CORNER_TABLE_T: [[(i32, i32); 4]; 4] = [[(-1, -1), (1, -1), (1, 1), (-1, 1)]; 4];

impl<const N: usize, const P: Piece> SpinMap<N, P> {
    /// Creates a new `SpinMap` from the given board.
    #[must_use]
    pub fn new(b: Board<N>) -> Self {
        let mut corners = [Board::<N>::EMPTY; 4];
        for r in 0..4 {
            for &(dx, dy) in &match P {
                Piece::T => CORNER_TABLE_T[r],
                Piece::J => CORNER_TABLE_J[r],
                Piece::L => CORNER_TABLE_L[r],
                Piece::S => CORNER_TABLE_S[r],
                Piece::Z => CORNER_TABLE_Z[r],

                _ => unreachable!(),
            } {
                corners[r] |= b.shifted(dx, dy);
            }
        }

        let front_corners = if P == Piece::T {
            let ul = b.shifted(-1, -1);
            let ur = b.shifted(1, -1);
            let dl = b.shifted(-1, 1);
            let dr = b.shifted(1, 1);
            let f_n = b & corners[0] & ul & ur;
            let f_e = b & corners[0] & ur & dr;
            let f_s = b & corners[0] & dl & dr;
            let f_w = b & corners[0] & ul & dl;
            [f_n, f_e, f_s, f_w]
        } else {
            [Board::<N>::EMPTY; 4]
        };
        let immobile =
            b & !b.shifted(0, -1) & !b.shifted(0, 1) & !b.shifted(-1, 0) & !b.shifted(1, 0);

        Self {
            corners,
            front_corners,
            immobile,
        }
    }
}
