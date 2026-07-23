//! Low-level move-generation operators used by the search routine.

use rue_core::board::Board;
use rue_core::data::KickTab;
use rue_core::piece::Piece;

#[inline]
/// Applies one kick candidate in-place, accumulating translated hits into `result`.
pub fn kick_step<const P: Piece, const D: usize, const R: usize, const I: usize, const N: usize>(
    temp: &mut Board<N>,
    result: &mut Board<N>,
    usable_r1c: &Board<N>,
) {
    let kx = i32::from(KickTab::<P, D, R>::ROW[I].0) + KickTab::<P, D, R>::OFF_X;
    let ky = i32::from(KickTab::<P, D, R>::ROW[I].1) + KickTab::<P, D, R>::OFF_Y;
    *result |= temp.shifted(kx, ky);

    if I != 4 {
        *temp &= !usable_r1c.shifted(-kx, -ky);
    }
}

#[inline]
#[must_use]
/// Expands a frontier by one horizontal step left/right within `usable` cells.
pub fn horizontal_tuck<const N: usize>(s: Board<N>, usable: &Board<N>) -> Board<N> {
    let left_right = (s.shifted(-1, 0) | s.shifted(1, 0)) & *usable;
    s | left_right
}

#[inline]
#[must_use]
/// Expands a blocking surface downward by powers of two up to `ceiling`.
pub fn vertical_ceiling<const N: usize>(mut surface: Board<N>, ceiling: i32) -> Board<N> {
    if ceiling >= 1 {
        surface |= surface.shifted(0, -1);
    }
    if ceiling >= 2 {
        surface |= surface.shifted(0, -2);
    }
    if ceiling >= 4 {
        surface |= surface.shifted(0, -4);
    }
    if ceiling >= 8 {
        surface |= surface.shifted(0, -8);
    }
    if ceiling >= 16 {
        surface |= surface.shifted(0, -16);
    }
    surface
}

#[macro_export]
/// Unrolls a compile-time rotation index loop for values `0..4` with a runtime limit.
macro_rules! unroll {
    ($r:ident, $limit:expr, $body:block) => {{
        {
            #[allow(non_upper_case_globals)]
            const $r: usize = 0;
            if $r < $limit $body
        }
        {
            #[allow(non_upper_case_globals)]
            const $r: usize = 1;
            if $r < $limit $body
        }
        {
            #[allow(non_upper_case_globals)]
            const $r: usize = 2;
            if $r < $limit $body
        }
        {
            #[allow(non_upper_case_globals)]
            const $r: usize = 3;
            if $r < $limit $body
        }
    }};
}

/// Drops a piece down until it collides with `usable` cells, returning the final resting position.
#[must_use]
#[inline]
pub fn sonic_drop<const N: usize>(s: Board<N>, usable: &Board<N>) -> Board<N> {
    let mut result = Board::<N>::EMPTY;
    let mut current = s;
    loop {
        let below = current.shifted(0, -1) & *usable;
        let moved = current & below.shifted(0, 1);
        let landed = current & !moved;
        result |= landed;
        current = below;
        if !current.any() { break; }
    }
    result
}