//! Main bot logic, including the bot struct, configuration, state management, and event handling.
//!
//! The bot connects to a server via triangle, joins or creates a room,
//! and listens for events such as chat messages and game updates.
//! It processes commands from users and manages behavior based on game state and configuration.
#![allow(clippy::missing_docs_in_private_items)]

mod bind;
mod burst;
pub mod state;
mod tick;

use std::fmt;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::sync::RwLock;

use triangle::Client;
use triangle::ClientOptions;
use triangle::classes::ribbon;
use triangle::types::events::recv;
use triangle::utils::EventEmitter;
use triangle::utils::api::core::ApiError;
use triangle::utils::events::WrapError;

use rue_core::board::Board;
use rue_core::game::Game;
use rue_core::game::garbage::GarbageQueue;
use rue_core::game::ruleset::SEASON_2;
use rue_core::piece::Piece;
use rue_core::rng::Rng;
use rue_core::rng::RngKind;
use rue_eval::simple::Simple;

use crate::bot::state::Config;
use crate::bot::state::EnabledState;
use crate::bot::state::Finesse;
use crate::bot::state::State;
use crate::command::registry::Registry;
use crate::command::traits::Restriction;
use crate::config::CONFIG;
use crate::env::env;
use crate::registry::{self};
use crate::settings::SettingsHandler;

/// Number of 6-row bands backing the live game board (42 rows).
const BOARD_BANDS: usize = 7;
/// The persistent solver-side game state kept for the live room.
type BotGame = Game<BOARD_BANDS>;



/// A join or create target for the bot.
#[derive(Debug, Clone)]
#[allow(missing_docs, clippy::missing_docs_in_private_items)]
pub enum Target {
    Join(String),
    Create,
}

/// The bot struct, containing the game state, configuration, client, and event handling.
pub struct Bot {
    game: Mutex<BotGame>,
    weights: Simple,
    /// The triangle client used to connect to the server and handle events.
    pub client: Client,
    /// The bot's configuration, including pieces per second (PPS), burst mode, and finesse style.
    pub config: RwLock<Config>,
    /// The bot's current state, including whether it is enabled, the current game state, and the current restriction level.
    pub state: RwLock<State>,
    /// The settings handler used to check room settings against constraints.
    pub settings: SettingsHandler,
    events: EventEmitter,
    /// The command registry used to handle chat commands.
    pub registry: Registry,
    /// The global configuration across *all* nodes, including search beam width and queue buffer size.
    pub global_config: crate::settings::Config,
}

/// An error that can occur when creating or running the bot, including connection errors, room errors, and IO errors.
#[derive(Debug)]
#[allow(missing_docs, clippy::missing_docs_in_private_items)]
pub enum BotError {
    Connection(ApiError),
    Room(WrapError),
    Io(std::io::Error),
}

impl fmt::Display for BotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BotError::Connection(err) => write!(f, "failed to create client: {err}"),
            BotError::Room(err) => write!(f, "failed to join or create room: {err}"),
            BotError::Io(err) => write!(f, "i/o error: {err}"),
        }
    }
}

impl std::error::Error for BotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BotError::Connection(err) => Some(err),
            BotError::Room(err) => Some(err),
            BotError::Io(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for BotError {
    fn from(err: std::io::Error) -> Self {
        BotError::Io(err)
    }
}

/// Appends `n` shuffled 7-bags to the end of the queue.
fn fill(queue: &mut Vec<Piece>, rng: &mut Rng, n: usize) {
    for _ in 0..n {
        let mut slice = RngKind::Bag7.slice();
        rng.shuffle_array(&mut slice);
        queue.extend_from_slice(&slice);
    }
}

impl Bot {
    /// Creates a new bot instance, connecting to the server and joining or creating a room based on the given target.
    pub async fn new(target: Target, global_config: crate::settings::Config) -> Result<Arc<Self>, BotError> {
        let client = Client::new(ClientOptions {
            game: Some(triangle::classes::GameOptions {
                handling: Some(CONFIG.handling),
                spectating_strategy: None,
            }),
            ribbon: Some(ribbon::OptionalParams {
                options: Some(ribbon::Options {
                    logging: ribbon::LoggingLevel::Error,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            social: None,
            token: triangle::Credentials::Token(env().token.clone()),
            user_agent: None,
        })
        .await
        .map_err(BotError::Connection)?;

        let (room_tx, room_rx) = tokio::sync::oneshot::channel::<recv::room::Update>();
        let room_tx = Arc::new(Mutex::new(Some(room_tx)));

        client.on::<recv::room::Update>(async move |data| {
            if let Some(tx) = room_tx.lock().await.take() {
                tx.send(data).ok();
            }
        });

        match target {
            Target::Join(roomid) => client.join_room(&roomid).await,
            Target::Create => client.create_room(false).await,
        }
        .map_err(BotError::Room)?;

        let room_update_data = room_rx
            .await
            .map_err(|_| BotError::Room(WrapError::ServerError))?;

        let mut registry = Registry::new();
        registry.register(Box::new(registry::info::ping_command));
        registry.register(Box::new(registry::info::help_command));
        registry.register(Box::new(registry::controls::kill_command));
        registry.register(Box::new(registry::controls::enable_command));
        registry.register(Box::new(registry::controls::disable_command));
        registry.register(Box::new(registry::controls::restrict_command));
        registry.register(Box::new(registry::controls::pps_command));
        registry.register(Box::new(registry::controls::burst_command));
        registry.register(Box::new(registry::controls::finesse_command));

        let weights =
            serde_json::from_str::<Simple>(&std::fs::read_to_string(env().weights.clone())?)
                .map_err(|e| BotError::Io(e.into()))?;

        let bot = Arc::new(Bot {
            global_config,
            // Real seeding happens once the room's queue seed is known, on round start.
            game: Mutex::new(Game {
                board: Board::EMPTY,
                hold: None,
                queue: Vec::new(),
                garbage_queue: GarbageQueue::new(),
                b2b_count: None,
                combo_count: None,
                ruleset: SEASON_2,
                rng: Rng::new(),
            }),
            weights,
            client,
            settings: SettingsHandler::new(),
            config: RwLock::new(Config {
                finesse: Finesse::Instant,
                pps: 1.0,
                burst: true,
                vision: 7,
            }),
            state: RwLock::new(State {
                enabled: EnabledState {
                    value: false,
                    attempt: true,
                    force: false,
                },
                game: None,
                restriction: Restriction::None,
            }),
            events: EventEmitter::new(),
            registry,
        });

        bot.handle_room_update(room_update_data, true).await;

        if let Some(room) = bot.client.room() {
            room.chat(":oyes:/").await.ok();
        } else {
            return Err(BotError::Room(WrapError::ServerError));
        }

        bot.bind().await;

        Ok(bot)
    }

    /// Destroys the bot, cleaning up resources and emitting a "close" event.
    pub async fn destroy(&self) {
        self.client.destroy().await;

        self.events.emit_raw("close", serde_json::json!({}));
        self.events.destroy();
    }
}
