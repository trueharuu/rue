use crate::command::traits::Restriction;

/// Finesse style for the bot. [`Finesse::Smooth`] is capped to 5 PPS.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[allow(missing_docs, clippy::missing_docs_in_private_items)]
pub enum Finesse {
    Instant,
    Smooth,
}

/// The bot's configuration, including pieces per second (PPS), burst mode, and finesse style.
#[derive(Debug, Clone)]
#[allow(missing_docs, clippy::missing_docs_in_private_items)]
pub struct Config {
    pub pps: f64,
    pub burst: bool,
    pub finesse: Finesse,
}
/// Whether the bot is enabled, and whether it should attempt to enable itself if disabled.
#[derive(Debug, Clone)]
pub struct EnabledState {
    /// Whether the bot is currently enabled.
    pub value: bool,
    /// Whether the bot should attempt to enable itself if disabled.
    pub attempt: bool,
    /// Whether the bot should forcefully enable itself, ignoring constraints.
    pub force: bool,
}

/// Current game state, including the last piece frame and the target frame for the next piece.
#[derive(Debug, Clone)]
pub struct GameState {
    pub last_piece_frame: u64,
    pub target_frame: u64,
}

/// Current room state, including whether the bot is enabled, the current game state, and the current restriction level.
#[derive(Debug, Clone)]
pub struct State {
    /// Whether the bot is enabled, and whether it should attempt to enable itself if disabled.
    pub enabled: EnabledState,
    /// The current game state.
    pub game: Option<GameState>,
    /// The current restriction level for commands in the room. Commands below this level will be ignored.
    pub restriction: Restriction,
}