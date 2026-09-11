use rue_core::game::attack::Attack;
use rue_core::game::search::SearchGame;
use rue_core::placement::Move;
use rue_core::rule::Rule;

pub trait Model {
    /// The name of the model.
    fn name(&self) -> &str;

    /// Evaluates a single placement on a game state, returning a scalar [`f32`] score.
    ///
    /// `game` holds the resulting board, queue, hold, chain, and incoming
    /// garbage state after `placement`. `ctx` is the attack produced by the
    /// placement.
    fn evaluate<const N: usize, const RULE: Rule>(
        &self,
        game: &SearchGame<N>,
        placement: Move,
        ctx: Attack,
    ) -> f32;
}