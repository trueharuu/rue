//! Move generation algorithms for pieces on a board.
//! This module generates all possible moves for a given piece on a given board
//! state. It respects the rules of the game and the current state of the board.

pub mod fast;
pub mod op;
pub mod oracle;
pub mod queue;
