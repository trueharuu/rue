//! Compact beam line state.

use rue_core::game::search::SearchGame;
use rue_core::placement::Move;

/// One line in the beam. Holds the simulated game state after the last
/// placement on this line.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Node<const N: usize> {
    /// The game after the last placement on this line.
    pub game: SearchGame<N>,
    /// The first placement of this line, inherited from the parent level.
    pub root_move: Move,
    /// Score of `game`.
    pub score: f32,
    /// Attack sent so far along this line, root to `game`.
    pub cum_attack: u32,
    /// Number of placements on this line, root to `game`.
    pub path_len: u32,
}
