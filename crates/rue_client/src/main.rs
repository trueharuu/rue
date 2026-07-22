//! Rue's TETR.IO client.
#![allow(unused)]

mod bot;
mod command;
mod config;
mod env;
mod events;
mod master;
mod settings;
mod utils;
mod registry;

#[tokio::main]
/// Entrypoint.
pub async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    env::parse_env();
    dotenvy::var("TOKEN").expect("TOKEN must be set in .env");

    master::Master::new()
        .await
        .expect("Failed to start master client");
    tokio::signal::ctrl_c().await.ok();
    events::events()
        .emit(events::msgs::Shutdown)
        .await;

    Ok(())
}
