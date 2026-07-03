//! Collision-derived occupancy maps used by move generation.

use rue_core::{board::Board, header::PCELLS, piece::Piece, rotation::Rotation};

#[inline]
#[must_use]
/// Returns origin positions where the `I`-th mino of `(P, R)` can be placed safely.
pub fn usable_cell<const P: Piece, const R: Rotation, const I: usize, const N: usize>(
    b: &Board<N>,
    nb: &Board<N>,
) -> Board<N> {
    let cx = i32::from(PCELLS[P as usize][R as usize][I].0);
    let cy = i32::from(PCELLS[P as usize][R as usize][I].1);
    if cy > 0 {
        (!b.shifted(0, -cy)).shifted(-cx, 0)
    } else {
        nb.shifted(-cx, -cy)
    }
}

#[inline]
#[must_use]
/// Returns origin positions where all four minos of `(P, R)` are collision-free.
pub fn usable_rot<const P: Piece, const R: Rotation, const N: usize>(
    b: &Board<N>,
    nb: &Board<N>,
) -> Board<N> {
    *nb & usable_cell::<P, R, 0, N>(b, nb)
        & usable_cell::<P, R, 1, N>(b, nb)
        & usable_cell::<P, R, 2, N>(b, nb)
}

#[inline]
#[must_use]
/// Builds usable-position maps for each canonical rotation of `P`.
pub fn usable_map<const P: Piece, const N: usize>(b: &Board<N>) -> [Board<N>; 4] {
    let negated = !*b;
    let mut usable = [Board::EMPTY; 4];
    usable[0] = usable_rot::<P, { Rotation::North }, N>(b, &negated);

    if P.canonical_rotations() > 1 {
        usable[1] = usable_rot::<P, { Rotation::East }, N>(b, &negated);
    }

    if P.canonical_rotations() > 2 {
        usable[2] = usable_rot::<P, { Rotation::South }, N>(b, &negated);
        usable[3] = usable_rot::<P, { Rotation::West }, N>(b, &negated);
    }

    usable
}

#[inline]
#[must_use]
/// Converts usable maps into landable maps by requiring support directly below.
pub fn landable_map<const N: usize>(u: &[Board<N>; 4], cs: usize) -> [Board<N>; 4] {
    let mut c = [Board::EMPTY; 4];
    macro_rules! land {
        ($r:literal) => {
            if $r < cs {
                c[$r] = u[$r] & !u[$r].shifted(0, 1);
            }
        };
    }

    land!(0);
    land!(1);
    land!(2);
    land!(3);
    c
}
