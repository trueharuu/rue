//! Master client logic, which manages child bot instances and handles invites.
use std::sync::Arc;

use tokio::sync::Mutex;
use triangle::Client;
use triangle::ClientOptions;
use triangle::classes::ribbon;
use triangle::types::events::recv;
use triangle::types::social::Detail;
use triangle::types::social::Status;
use triangle::utils::api::core::ApiError;

use crate::bot::Bot;
use crate::bot::Target;
use crate::env::env;
use crate::events::events;
use crate::events::msgs;
use crate::settings::Config;

/// A global master client that manages child bot instances and handles invites.
pub struct Master {
    /// The triangle client used to connect to the server and handle events.
    client: Client,
    /// All connected children currently in rooms.
    children: Arc<Mutex<Vec<Arc<Bot>>>>,
    /// The global configuration across *all* nodes.
    config: Config,
}

impl Master {
    /// Creates a new master client, connecting to the server and setting up event handlers.
    pub async fn new(cfg: Config) -> Result<Self, ApiError> {
        let c = Master {
            client: Client::new(ClientOptions {
                game: None,
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
            .await?,
            children: Arc::new(Mutex::new(Vec::new())),
            config: cfg,
        };

        c.init().await;

        Ok(c)
    }

    /// Initializes the master client, setting its status and registering event handlers for invites,
    /// DMs, and shutdown events.
    async fn init(&self) {
        self.client
            .social
            .set_status(Status::Online, Detail::Zen)
            .await;

        let c = self.children.clone();
        let cc = self.client.clone();
        let config = self.config.clone();

        self.client.on::<recv::social::Invite>(async move |invite| {
            match Bot::new(Target::Join(invite.roomid.clone()), config).await {
                Ok(bot) => {
                    c.lock().await.push(bot);
                }
                Err(e) => {
                    let message = e.to_string();
                    cc.social
                        .dm(invite.sender, format!("failed to join room: {message}"))
                        .await
                        .ok();
                }
            }
        });

        let c = self.children.clone();

        // join dev room
        if let Some(ref id) = self.config.dev_room_id {
            match Bot::new(Target::Join(id.clone()), self.config.clone()).await {
                Ok(bot) => {
                    c.lock().await.push(bot);
                }
                Err(e) => {
                    tracing::error!("failed to join dev room {id}: {e}");
                }
            }
        }

        let client = self.client.clone();

        // TODO: handle DM commands

        let mut client = self.client.clone();

        self.client.on::<recv::client::Dead>(async move |_| {
            loop {
                if client.reconnect().await.is_ok() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        });

        let client = self.client.clone();

        events()
            .on::<msgs::Shutdown>(move |_| {
                let client = client.clone();
                async move {
                    client
                        .social
                        .set_status(Status::Offline, Detail::Menus)
                        .await;
                    client.destroy().await;
                }
            })
            .await;
    }
}
