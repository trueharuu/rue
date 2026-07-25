//! Low-level move-generation operators used by the search routine.

use rue_core::board::Board;
use rue_core::data::K5;

#[inline]
/// Applies one kick candidate in-place, accumulating translated hits into `result`.
pub fn kick_step<const I: usize, const N: usize>(
    temp: &mut Board<N>,
    result: &mut Board<N>,
    usable_r1c: &Board<N>,
    kick_row: &K5,
    off_x: i32,
    off_y: i32,
) {
    let kx = i32::from(kick_row[I].0) + off_x;
    let ky = i32::from(kick_row[I].1) + off_y;
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

/// Projects a frontier straight down to its resting positions within `usable` cells.
///
/// Every set bit in `s` falls until it hits the floor or an occupied cell,
/// i.e. until it lands on a cell in `landable_map`. Used for sonic-drop mode,
/// where the piece never pauses mid-air between tucks/rotations.
#[inline]
#[must_use]
pub fn sonic_drop<const N: usize>(mut s: Board<N>, usable: &Board<N>) -> Board<N> {
    loop {
        let new = s.shifted(0, -1) & *usable & !s;
        if !new.any() {
            break;
        }
        s |= new;
    }
    s
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
