//! Rue's TETR.IO client.
pub mod command;
pub mod game;
pub mod root;
pub mod util;

use triangle::ClientOptions;

use crate::command::core::{context::Context, registry::Registry};

#[tokio::main]
/// Entrypoint.
pub async fn main() -> anyhow::Result<()> {
    let token = dotenvy::var("TOKEN").expect("TOKEN must be set in .env");
    let dev_room_id = dotenvy::var("DEV_ROOM_ID").expect("DEV_ROOM_ID must be set in .env");
    let client = triangle::Client::new(ClientOptions::with_token(token)).await?;

    client.join_room(&dev_room_id).await?;

    let mut registry = Registry::new();

    registry.register(Box::new(command::info::ping_command));

    // todo: wire up message reception from triangle, parse command name,
    // look up in registry, dispatch with Context, and forward replies
    // from the channel back to the room

    root::master::Master::new()
        .await
        .expect("Failed to start master client");
    tokio::signal::ctrl_c().await.ok();
    util::events::events()
        .emit(util::events::msgs::Shutdown)
        .await;

    Ok(())
}

/// Dispatch a raw message string through the registry.
#[allow(dead_code)]
async fn dispatch(
    registry: &Registry,
    text: &str,
    reply_tx: &tokio::sync::mpsc::Sender<String>,
) -> anyhow::Result<()> {
    let mut parts = text.splitn(2, char::is_whitespace);
    let cmd_name = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("");

    match registry.find(cmd_name) {
        Some(cmd) => {
            let mut ctx = Context::new(args, reply_tx);
            cmd.execute(&mut ctx).await
        }
        None => Err(anyhow::anyhow!("unknown command: {cmd_name}")),
    }
}
