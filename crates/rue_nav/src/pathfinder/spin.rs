//! Spin classification methods.

use rue_core::board::Board;
use rue_core::game::ruleset::Handling;
use rue_core::header::WIDTH;
use rue_core::piece::Piece;
use rue_core::spin::Spin;
use rue_core::spin::Spins;

/// Returns true if the given placement is immobile (cannot move in any of the four directions).
#[must_use]
#[inline]
pub fn immobile<const N: usize>(cm: &[Board<N>; 4], x: i32, y: i32, rt: usize) -> bool {
    let l = x == 0 || !cm[rt].get(x - 1, y);
    let r = x == (WIDTH - 1) || !cm[rt].get(x + 1, y);
    let u = y == 0 || !cm[rt].get(x, y - 1);
    let d = !cm[rt].get(x, y + 1);
    l && r && u && d
}

/// Classifies the [`Spin`]-type of a placement.
#[must_use]
#[inline]
pub fn classify<const P: Piece, const RULE: Handling, const N: usize>(
    b: &Board<N>,
    cm: &[Board<N>; 4],
    x: i32,
    y: i32,
    r: usize,
    kick_idx: usize,
) -> Spin {
    if RULE.spins == Spins::None {
        return Spin::None;
    }

    let stuck = immobile::<N>(cm, x, y, r);

    if P == Piece::T {
        let corner = |dx: i32, dy: i32| -> bool {
            let cx = x + dx;
            let cy = y + dy;
            !(0..WIDTH).contains(&cx) || cy < 0 || b.get(cx, cy)
        };

        let nw = corner(-1, 1);
        let ne = corner(1, 1);
        let sw = corner(-1, -1);
        let se = corner(1, -1);

        let spins = (nw && ne && (se || sw)) || (se && sw && (nw || ne));
        if !spins && !stuck {
            return Spin::None;
        }

        if kick_idx >= 4 {
            return if spins { Spin::Full } else { Spin::Mini };
        }

        let front = match r {
            0 => nw && ne,
            1 => ne && se,
            2 => se && sw,
            3 => sw && nw,
            _ => unreachable!(),
        };

        if spins && front {
            return Spin::Full;
        }

        return Spin::Mini;
    }

    if RULE.spins.has_immobile() && stuck {
        match RULE.spins {
            Spins::AllMini => return Spin::Mini,
            Spins::AllPlus => return Spin::Full,
            _ => {},
        }
    }

    Spin::None
}
