//! Main bot logic.
//! Contains the bot struct, configuration, state management, and event handling.
//!
//! The bot connects to a server through triangle. It joins or creates a room.
//! It listens for events such as chat messages and game updates.
//! It processes user commands and manages behavior from game state and
//! configuration.
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

/// A join or create target for the bot.
#[derive(Debug, Clone)]
#[allow(missing_docs, clippy::missing_docs_in_private_items)]
pub enum Target {
    Join(String),
    Create,
}

/// The bot struct.
/// Contains the game state, configuration, client, and event handling.
pub struct Bot {
    /// The triangle client used to connect to the server and handle events.
    pub client: Client,
    /// The bot configuration.
    /// Includes pieces per second (PPS), burst mode, and finesse style.
    pub config: RwLock<Config>,
    /// The current bot state.
    /// Includes whether the bot is enabled, the game state, and the restriction level.
    pub state: RwLock<State>,
    /// The settings handler used to check room settings against constraints.
    pub settings: SettingsHandler,
    events: EventEmitter,
    /// The command registry used to handle chat commands.
    pub registry: Registry,
    /// The global configuration across all nodes.
    /// Includes search beam width and queue buffer size.
    pub global_config: crate::settings::Config,
}

/// An error that can occur when creating or running the bot.
/// Includes connection, room, and IO errors.
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

impl Bot {
    /// Creates a new bot instance, connecting to the server and joining or creating a room based on the given target.
    pub async fn new(
        target: Target,
        global_config: crate::settings::Config,
    ) -> Result<Arc<Self>, BotError> {
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

        let bot = Arc::new(Bot {
            global_config,
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
