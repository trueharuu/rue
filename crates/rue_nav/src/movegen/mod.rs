//! Move generation algorithms for pieces on a board.
//! This module contains the core logic for generating all possible moves for a
//! given piece on a given board state, taking into account the rules of the
//! game and the current state of the board.

pub mod fast;
pub mod op;
pub mod oracle;
pub mod queue;
