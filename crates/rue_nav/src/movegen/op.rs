use std::simd::Simd;

use rue_core::board::Board;
use rue_core::data::CELLS;
use rue_core::data::KICKS_I;
use rue_core::data::KICKS_O;
use rue_core::data::KICKS_TJLSZ;
use rue_core::header::COL0;
use rue_core::header::COL9;
use rue_core::header::TALL;
use rue_core::header::WIDTH;
use rue_core::piece::Piece;
use rue_core::placement::Move;
use rue_core::rot_idx;
use rue_core::rotation::Rotation;
use rue_core::rule::Rule;
use rue_core::spin::Spin;
use rue_core::spin::Spins;

/// Returns whether a piece with rotation index `r` placed at cell `(x, y)` is
/// inside the board and does not overlap any occupied cell.
#[inline]
#[must_use]
pub fn check<const N: usize, const P: Piece>(board: &Board<N>, x: i32, y: i32, r: usize) -> bool {
    if !(0..WIDTH).contains(&x) || !(0..Board::<N>::total_height()).contains(&y) {
        return false;
    }

    let mv = Move::new(P, x, y, Rotation::from_u8(r as u8), Spin::None);

    for (x, y) in mv.cells() {
        if !(0..WIDTH).contains(&x)
            || !(0..Board::<N>::total_height()).contains(&y)
            || board.get(x, y)
        {
            return false;
        }
    }

    true
}

/// [`check`], using a precomputed [`usable_map`] to determine valid placements.
#[inline]
#[must_use]
pub fn check_fast<const N: usize, const P: Piece>(
    usable: &[Board<N>; Rotation::NB],
    x: i32,
    y: i32,
    r: usize,
) -> bool {
    // canonicalize the position and rotation
    let (dx, dy) = P.canonical_offset(rot_idx!(r));
    let r = P.canonical_rotation(rot_idx!(r)) as usize;
    if !(0..WIDTH).contains(&(x - dx)) || !(0..Board::<N>::total_height()).contains(&(y - dy)) {
        return false;
    }

    usable[r].get(x - dx, y - dy)
}

/// Returns origin positions where the `I`-th mino of `(P, R)` can be placed
/// safely.
#[inline]
#[must_use]
pub fn usable_cell<const N: usize, const P: Piece, const R: Rotation, const I: usize>(
    b: &Board<N>,
    nb: &Board<N>,
) -> Board<N> {
    let cx = i32::from(CELLS[P as usize][R as usize][I].0);
    let cy = i32::from(CELLS[P as usize][R as usize][I].1);
    if cy > 0 {
        (!b.shifted(0, -cy)).shifted(-cx, 0)
    } else {
        nb.shifted(-cx, -cy)
    }
}

/// Returns origin positions where all four minos of `x(P, R)` are
/// collision-free.
#[inline]
#[must_use]
pub fn usable_rot<const N: usize, const P: Piece, const R: Rotation>(
    b: &Board<N>,
    nb: &Board<N>,
) -> Board<N> {
    *nb & usable_cell::<N, P, R, 0>(b, nb)
        & usable_cell::<N, P, R, 1>(b, nb)
        & usable_cell::<N, P, R, 2>(b, nb)
}

/// Builds usable-position maps for every canonical rotation of `P`.
#[inline]
#[must_use]
pub fn usable_map<const N: usize, const P: Piece>(b: &Board<N>) -> [Board<N>; Rotation::NB] {
    let negated = !*b;
    let mut usable = [Board::empty(); Rotation::NB];
    usable[0] = usable_rot::<N, P, { Rotation::North }>(b, &negated);

    if P.group2() || P.group4() {
        usable[1] = usable_rot::<N, P, { Rotation::East }>(b, &negated);
    }

    if P.group4() {
        usable[2] = usable_rot::<N, P, { Rotation::South }>(b, &negated);
        usable[3] = usable_rot::<N, P, { Rotation::West }>(b, &negated);
    }

    usable
}

/// Converts usable maps into landable maps by requiring support directly below.
#[inline]
#[must_use]
pub fn landable_map<const N: usize>(
    u: &[Board<N>; Rotation::NB],
    cs: usize,
) -> [Board<N>; Rotation::NB] {
    let mut c = [Board::empty(); Rotation::NB];
    for r in 0..cs {
        c[r] = u[r] & !u[r].shifted(0, 1);
    }
    c
}

/// Attempts to rotate `mv` to `target`, applying the first valid kick; returns
/// the original move when no kick succeeds.
#[inline]
#[must_use]
pub fn apply_rotation<const N: usize, const P: Piece, const RULE: Rule>(
    board: &Board<N>,
    usable: &[Board<N>; Rotation::NB],
    mv: &Move,
    target: Rotation,
) -> Move {
    let rotate_o_meaningful = RULE.spins == Spins::Stupid;
    if mv.rotation() == target || (!rotate_o_meaningful && mv.piece() == Piece::O) {
        return *mv;
    }

    let kt = match mv.piece() {
        Piece::I => KICKS_I,
        Piece::O => KICKS_O,
        _ => KICKS_TJLSZ,
    };

    let lane = kt[mv.rotation() as usize][target as usize];
    for kick_idx in 0..lane.1 {
        let dx = i32::from(lane.0[kick_idx].0);
        let dy = i32::from(lane.0[kick_idx].1);
        let new_x = mv.x() + dx;
        let new_y = mv.y() + dy;
        if check_fast::<N, P>(usable, new_x, new_y, target as usize) {
            return Move::new(
                mv.piece(),
                new_x,
                new_y,
                target,
                classify::<N, P, RULE>(board, usable, new_x, new_y, target as usize, kick_idx),
            );
        }
    }

    *mv
}

/// Classifies the spin outcome for a piece placed at cell `(x, y)` with
/// rotation `r`.
///
/// Assumes that this placement was reached directly via rotation and is a
/// landed position.
#[inline]
#[must_use]
pub fn classify<const N: usize, const P: Piece, const RULE: Rule>(
    board: &Board<N>,
    usable: &[Board<N>; 4],
    x: i32,
    y: i32,
    r: usize,
    kick_idx: usize,
) -> Spin {
    // everything isn't spin in None
    if RULE.spins == Spins::None {
        return Spin::None;
    }

    // all spins are spin in Stupid
    if RULE.spins == Spins::Stupid {
        return Spin::Full;
    }

    // bare minimum 3-corner T spin detection
    let is_t = P == Piece::T;
    let up_left = x == 0 || board.get(x - 1, y + 1);
    let up_right = x == WIDTH - 1 || board.get(x + 1, y + 1);
    let down_left = x == 0 || y == 0 || board.get(x - 1, y - 1);
    let down_right = x == WIDTH - 1 || y == 0 || board.get(x + 1, y - 1);
    let has_3 =
        (u8::from(up_left) + u8::from(up_right) + u8::from(down_left) + u8::from(down_right)) >= 3;
    let front_corners = match r {
        0 => up_left && up_right,
        1 => up_right && down_right,
        2 => down_left && down_right,
        3 => up_left && down_left,
        _ => unreachable!(),
    };

    if is_t && RULE.has_t_corner_spins() {
        if has_3 {
            if front_corners || kick_idx >= 4 {
                return Spin::Full;
            }

            // even in all-mini and all, t-minis are still mini
            return Spin::Mini;
        }

        return Spin::None;
    }

    let up = check_fast::<N, P>(usable, x, y + 1, r);
    let down = check_fast::<N, P>(usable, x, y - 1, r);
    let left = check_fast::<N, P>(usable, x - 1, y, r);
    let right = check_fast::<N, P>(usable, x + 1, y, r);

    let is_immobile = !up && !down && !left && !right;

    if is_immobile && RULE.has_immobile_t_spins() && is_t {
        return Spin::Mini;
    }

    if is_immobile && RULE.has_immobile_non_t_spins() && !is_t {
        if RULE.is_full() {
            return Spin::Full;
        }

        return Spin::Mini;
    }

    Spin::None
}

#[macro_export]
/// Unrolls a compile-time rotation index loop for values `0..4` with a runtime
/// limit.
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

/// Origins reached by one `SoftDrop` input from any origin in `s`.
///
/// - `inf_sdf = false`: exactly one row down, into `usable` cells.
/// - `inf_sdf = true`: the resting cell at the bottom of each usable column-run
///   the frontier touches (the piece falls all the way in a single input).
#[inline]
#[must_use]
pub fn vertical_drop<const N: usize, const RULE: Rule>(s: Board<N>, usable: &Board<N>) -> Board<N> {
    if const { RULE.inf_sdf } {
        let resting = *usable & !usable.shifted(0, 1);
        sonic_drop(s, usable) & resting
    } else {
        s.shifted(0, -1) & *usable
    }
}

/// Downward closure of `s` within `usable` (all cells the frontier falls
/// through).
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

/// Expands a blocking surface downward by powers of two up to `ceiling`. Runs
/// in O(log(ceiling)) time rather than O(ceiling) time.
#[inline]
#[must_use]
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

/// Expands a frontier by one horizontal step left/right within `usable` cells.
#[inline]
#[must_use]
pub fn horizontal_tuck<const N: usize>(s: Board<N>, usable: &Board<N>) -> Board<N> {
    const ML: u64 = TALL & !COL9;
    const MR: u64 = TALL & !COL0;
    let mut result = Simd::splat(0);
    for i in 0..N {
        let w = s.0[i];
        result[i] = ((w << 1) & ML) | ((w >> 1) & MR);
    }
    
    Board(result) & *usable
}
