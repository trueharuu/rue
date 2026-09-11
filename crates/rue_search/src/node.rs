//! Compact beam line state.

use rue_core::game::Game;
use rue_core::placement::Move;
use rue_core::rule::Rule;

/// One line in the beam. Holds the simulated game state after the last
/// placement on this line.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Node<const N: usize, const RULE: Rule> {
    /// The game after the last placement on this line.
    pub game: Game<N, RULE>,
    /// The first placement of this line, inherited from the parent level.
    pub root_move: Move,
    /// Score of `game`.
    pub score: f32,
}