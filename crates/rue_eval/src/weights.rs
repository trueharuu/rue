//! `Weights` trait.

use rue_core::game::{Game, attack::AttackContext};

/// A series of weights.
pub trait Weights: Sized + Send + Sync {
    /// The name of the model.
    fn name() -> &'static str;

    /// Evaluate a singular position on the board and return a score.
    fn evaluate<const N: usize>(&self, game: &Game<N>, context: &AttackContext) -> f64;

    /// Unique, time-based hash for weight files.
    #[must_use]
    fn hash() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        format!("{}-{:x}", Self::name(), since_the_epoch.as_secs())
    }
}
