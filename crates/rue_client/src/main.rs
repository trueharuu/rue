//! Rue's TETR.IO client.
#![allow(unused)]

use crate::settings::Config;

mod bot;
mod command;
mod config;
mod env;
mod events;
mod master;
mod registry;
mod settings;
mod utils;

#[tokio::main]
/// Entrypoint.
pub async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    env::parse_env();
    dotenvy::var("TOKEN").expect("TOKEN must be set in .env");

    let prefix = dotenvy::var("PREFIX").expect("PREFIX must be set in .env");
    let search_beam_width = dotenvy::var("SEARCH_BEAM_WIDTH")
        .expect("SEARCH_BEAM_WIDTH must be set in .env")
        .parse::<usize>()
        .expect("SEARCH_BEAM_WIDTH must be a valid usize");
    let queue_lookahead = dotenvy::var("QUEUE_LOOKAHEAD")
        .expect("QUEUE_LOOKAHEAD must be set in .env")
        .parse::<usize>()
        .expect("QUEUE_LOOKAHEAD must be a valid usize");
    let name = dotenvy::var("NAME").expect("NAME must be set in .env");

    let hosts = dotenvy::var("HOSTS")
        .expect("HOSTS must be set in .env")
        .split(',')
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();

    let dev_room_id = dotenvy::var("DEV_ROOM_ID").ok();

    let config = Config {
        prefix,
        name,
        search_beam_width,
        queue_buffer: queue_lookahead,
        hosts,
        dev_room_id,
    };

    master::Master::new(config)
        .await
        .expect("failed to start master client");
    tokio::signal::ctrl_c().await.ok();
    events::events().emit(events::msgs::Shutdown).await;

    Ok(())
}
