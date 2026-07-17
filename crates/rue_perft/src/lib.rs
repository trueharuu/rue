//! Perft (performance test) recursive tree search driver.
//!
//! This module computes the total number of leaf nodes reachable from an empty board
//! over a given piece queue, using dynamic band-width selection to minimize memory usage
//! and height caching to avoid redundant full-board rescans.
#![feature(min_adt_const_params)]

mod dispatch;
pub mod fusion;
pub mod height;
mod traversal;

pub use dispatch::perft_rec;
use rue_core::board::Board;
use rue_core::piece::Piece;
use rue_core::placement::Move;
use rue_core::rotation::Rotation;
use rue_core::spin::{Spin, Spins};
use rue_nav::movegen::generate_inlined;

/// Perft from an empty board over the given piece queue.
///
/// Returns the total number of reachable leaf nodes.
#[must_use]
pub fn perft(queue: &[Piece]) -> u64 {
    let b = Board::<8>::EMPTY;
    perft_rec(&b, queue, queue.len(), 0)
}

/// Multithreaded perft over disjoint subtree work items.
///
/// Splits the first plies into child boards sequentially, then fans the
/// disjoint subtrees out across a rayon pool. Each subtree runs the
/// unchanged sequential driver, so the total is identical to `perft` —
/// addition over disjoint subtrees is order-independent.
#[must_use]
pub fn perft_mt(queue: &[Piece]) -> u64 {
    use rayon::prelude::*;

    let depth = queue.len();

    // Splitting overhead exceeds the work at trivial depths.
    if depth <= 2 {
        return perft(queue);
    }

    // Three split plies gives a few thousand work items at depth 7
    // (IOL = 5266), fine-grained enough for work stealing to balance
    // uneven subtree sizes while collection stays negligible.
    let split = (depth - 1).min(3);
    let mut work: Vec<(Board<8>, i32)> = vec![(Board::<8>::EMPTY, 0)];

    for &p in &queue[..split] {
        let mut next = Vec::with_capacity(work.len() * 24);
        for (b, h) in &work {
            collect_children(b, *h, p, &mut next);
        }
        work = next;
    }

    let rest = &queue[split..];
    work.par_iter()
        .map(|(b, h)| perft_rec(b, rest, depth - split, *h))
        .sum()
}

/// Enumerate the child boards of `b` for piece `p` with exact heights.
/// Cold path: runs once per work-list ply, so plain full-band ops suffice.
fn collect_children(b: &Board<8>, h: i32, p: Piece, out: &mut Vec<(Board<8>, i32)>) {
    fn go<const P: Piece>(b: &Board<8>, h: i32, out: &mut Vec<(Board<8>, i32)>) {
        let ml = generate_inlined::<P, { Spins::None }, 8>(b, h, 0);
        let mut rc = 0;
        while rc < P.canonical_rotations() {
            ml.none[rc].for_each_set_bit(|x, y| {
                let mut b2 = *b;
                b2.do_move(Move::new(P, Rotation::from(rc as u8), x, y, Spin::None));
                out.push((b2, b2.max_y()));
            });
            rc += 1;
        }
    }

    match p {
        Piece::I => go::<{ Piece::I }>(b, h, out),
        Piece::O => go::<{ Piece::O }>(b, h, out),
        Piece::T => go::<{ Piece::T }>(b, h, out),
        Piece::J => go::<{ Piece::J }>(b, h, out),
        Piece::L => go::<{ Piece::L }>(b, h, out),
        Piece::S => go::<{ Piece::S }>(b, h, out),
        Piece::Z => go::<{ Piece::Z }>(b, h, out),
    }
}
