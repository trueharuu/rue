//! Piece type dispatch and entry point to recursive search.

use rue_core::board::Board;
use rue_core::piece::Piece;

use super::traversal::with_piece;

/// Top-level dispatch by piece type.
///
/// Routes the next piece in the queue to the appropriate const-generic handler.
#[must_use]
pub fn perft_rec(b: &Board<8>, q: &[Piece], depth: usize, h: i32) -> u64 {
    if q.is_empty() {
        return 0;
    }
    match q[0] {
        Piece::I => with_piece::<{ Piece::I }>(b, q, depth, h),
        Piece::O => with_piece::<{ Piece::O }>(b, q, depth, h),
        Piece::T => with_piece::<{ Piece::T }>(b, q, depth, h),
        Piece::J => with_piece::<{ Piece::J }>(b, q, depth, h),
        Piece::L => with_piece::<{ Piece::L }>(b, q, depth, h),
        Piece::S => with_piece::<{ Piece::S }>(b, q, depth, h),
        Piece::Z => with_piece::<{ Piece::Z }>(b, q, depth, h),
    }
}
