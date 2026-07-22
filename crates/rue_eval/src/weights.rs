//! `Weights` trait.

use rue_core::game::Game;
use rue_core::game::attack::AttackContext;
use rue_core::placement::Move;

/// A series of weights.
pub trait Weights: Sized + Send + Sync {
    /// The name of the model.
    fn name() -> &'static str;

    /// Evaluate a singular position on the board and return a score.
    fn evaluate<const N: usize>(&self, game: &Game<N>, context: &AttackContext) -> f64;

    /// Evaluate with piece history reconstructed from the search path.
    /// By default ignores history, delegating to [`Weights::evaluate`].
    fn evaluate_with_path<const N: usize>(
        &self,
        game: &Game<N>,
        context: &AttackContext,
        path: &[Move],
    ) -> f64 {
        let _ = path;
        self.evaluate(game, context)
    }

    /// Unique, time-based hash for weight files.
    #[must_use]
    fn hash() -> String {
        use std::time::SystemTime;
        use std::time::UNIX_EPOCH;
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        format!("{}-{:x}", Self::name(), since_the_epoch.as_secs())
    }
}
