//! Utility functions for the Rue client.

use rue_core::rng::Rng;
use rue_nav::pathfinder::Input;
use triangle::types::game::Key;

/// A single raw controller action for the current piece: either a hold
/// (decided by the bot before pathfinding runs) or a step in the path
/// returned by [`rue_nav::pathfinder::get_input`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs, clippy::missing_docs_in_private_items)]
pub enum BotMove {
    Hold,
    Path(Input),
}

/// Calculates the number of frames until the next piece spawns,
/// given the number of pieces that have spawned, the current pieces per second (PPS),
/// the time since the last piece spawned, and the minimum and maximum PPS values.
pub fn frames_till_next_piece(pieces: u32, pps: f64, time: f64, min_pps: f64, max_pps: f64) -> f64 {
    let res = ((f64::from(pieces) + 1.0) / pps) * 60.0 - time;
    let lower = 60.0 / max_pps;
    let upper = 60.0 / min_pps;
    lower.max(upper.min(res))
}

/// Generates a normally distributed random number with the given mean and standard deviation.
pub fn normal_random(mean: f64, stdev: f64) -> f64 {
    let mut rng = Rng::new();
    loop {
        let u: f64 = rng.next_float() * 2.0 - 1.0;
        let v: f64 = rng.next_float() * 2.0 - 1.0;
        let s = u * u + v * v;
        if s < 1.0 && s != 0.0 {
            let z = u * ((-2.0 * s.ln()) / s).sqrt();
            return z * stdev + mean;
        }
    }
}

/// Converts a [`BotMove`] to a [`Key`] for sending to the server.
pub fn move_to_key(m: BotMove) -> Key {
    match m {
        BotMove::Hold => Key::Hold,
        BotMove::Path(Input::ShiftLeft | Input::DasLeft) => Key::MoveLeft,
        BotMove::Path(Input::ShiftRight | Input::DasRight) => Key::MoveRight,
        BotMove::Path(Input::SoftDrop) => Key::SoftDrop,
        BotMove::Path(Input::HardDrop) => Key::HardDrop,
        BotMove::Path(Input::RotateCW) => Key::RotateCW,
        BotMove::Path(Input::RotateCCW) => Key::RotateCCW,
        BotMove::Path(Input::RotateFlip) => Key::Rotate180,
    }
}

/// Success symbol.
pub const SUCCESS: &str = "✅";
/// Failure symbol.
pub const FAILURE: &str = "❌";
/// Warning symbol.
pub const WARNING: &str = "⚠️";
