//! Rue's TETR.IO client.
pub mod command;
pub mod game;
pub mod root;
pub mod util;

#[tokio::main]
/// Entrypoint.
pub async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    util::env::parse_env();
    dotenvy::var("TOKEN").expect("TOKEN must be set in .env");

    root::master::Master::new()
        .await
        .expect("Failed to start master client");
    tokio::signal::ctrl_c().await.ok();
    util::events::events()
        .emit(util::events::msgs::Shutdown)
        .await;

    Ok(())
}
