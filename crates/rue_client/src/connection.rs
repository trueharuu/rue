use std::sync::{Arc};
use tokio::sync::Mutex;

use triangle::{Client, ClientOptions};
use tracing::{warn};

pub struct Bot {
    pub client: Client,
}

impl Bot {
    pub async fn new(token: &str, code: &str) -> Self {
        let client = Client::new(ClientOptions::with_token(token)).await.unwrap();

        client.join_room(code).await.unwrap();

        Bot { client }
    }
}

pub struct Master {
    pub token: String,
    pub client: Client,
    pub children: Arc<Mutex<Vec<Bot>>>,
}

impl Master {
    pub async fn new(token: String) -> Self {
        let client = Client::new(ClientOptions::with_token(&token)).await.unwrap();
        let token_for_invites = token.clone();
        let children = Arc::new(Mutex::new(Vec::new()));
        let children_for_invite = children.clone();
        client.on::<triangle::types::events::recv::social::Invite>(async move |invite: triangle::types::events::recv::social::Invite| {
            let bot = Bot::new(&token_for_invites, &invite.roomid).await;
            children_for_invite.lock().await.push(bot);
        });

        Master { token, client, children }
    }

    pub async fn start(self) {
        // info!("master ready; waiting for invites");
        let _ = tokio::signal::ctrl_c().await;
        warn!("shutdown");
        for bot in self.children.lock().await.iter() {
            if let Some(s) = bot.client.room() {
                let id = s.state().await.id;
                s.leave().await;
                warn!("left {id}");
            }
        }
    }
}
