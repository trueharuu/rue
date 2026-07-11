//! Rue's TETR.IO client.
pub mod command;

use std::sync::Arc;

use triangle::{Client, ClientOptions};

use crate::command::core::{context::Context, registry::Registry};

#[tokio::main]
/// Entrypoint.
pub async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // todo: make these a `Config` struct idk
    let token = dotenvy::var("TOKEN").expect("TOKEN must be set in .env");
    let prefix = dotenvy::var("PREFIX").expect("PREFIX must be set in .env");
    let dev_room_id = dotenvy::var("DEV_ROOM_ID").ok();
    let _hosts = dotenvy::var("HOSTS")
        .expect("HOSTS must be set in .env")
        .split(',')
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let client = triangle::Client::new(ClientOptions::with_token(token)).await?;

    if let Some(dev_room_id) = dev_room_id {
        tracing::info!("joining dev room {dev_room_id}");
        client.join_room(&dev_room_id).await?;
    } else {
        tracing::warn!("DEV_ROOM_ID not set, skipping dev room join");
    }

    let mut registry = Registry::new();
    registry.register(Box::new(command::info::ping_command));

    let registry_arc = Arc::new(registry);
    let prefix_arc = Arc::new(prefix.clone()); // stare
    let client_arc = Arc::new(client.clone());

    if client.room().is_some() {
        client.on::<triangle::types::events::recv::room::Chat>(async |c| {
            if let Err(e) = dispatch(prefix_arc, client_arc, registry_arc, &c).await {
                tracing::warn!("failed to dispatch command: {e}");
            }
        });
    }

    tokio::signal::ctrl_c().await?;
    
    // leave room
    if let Some(room) = client.room() {
        room.leave().await;
    }

    client.destroy().await;

    Ok(())
}

/// Dispatch a raw message string through the registry.
#[allow(dead_code)]
async fn dispatch(
    prefix: Arc<String>,
    client: Arc<Client>,
    registry: Arc<Registry>,
    event: &triangle::types::events::recv::room::Chat,
) -> anyhow::Result<()> {
    let Some(text) = event.content.trim().strip_prefix(prefix.as_str()) else {
        return Ok(());
    };
    let mut parts = text.splitn(2, char::is_whitespace);
    let cmd_name = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("");

    if let Some(cmd) = registry.find(cmd_name) {
        let room_opt = client.room();
        let room = room_opt.as_ref().unwrap_or_else(|| todo!());
        let mut ctx = Context::new(args, event, room, client.as_ref());
        cmd.execute(&mut ctx).await
    } else {
        // Err(anyhow::anyhow!("unknown command: {cmd_name}"))
        tracing::warn!("unknown command: {cmd_name}");
        Ok(())
    }
}
