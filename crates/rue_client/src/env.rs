//! Environment configuration for the TETR.IO client.
use std::sync::OnceLock;

/// Global environment configuration for the TETR.IO client.
/// Contains the API token and the weights path.
pub struct Env {
    /// API token for authenticating with the TETR.IO API.
    pub token: String,
    /// Path to the weights file.
    pub weights: String,
    /// Current bot prefix.
    pub prefix: String,
    /// Bot hosts. A host has elevated permissions for managing the bot.
    pub hosts: Vec<String>,
    /// Development room ID, if any.
    /// Rue joins this room immediately at startup.
    pub dev_room: Option<String>,
}

/// Default weights path.
/// Resolved from the source tree at compile time.
/// It does not depend on the working directory of the binary.
const DEFAULT_WEIGHTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../weights/simple.json");

/// Global environment configuration.
static ENV: OnceLock<Env> = OnceLock::new();

/// Returns a reference to the global environment configuration. Panics if the environment has not been initialized.
pub fn env() -> &'static Env {
    ENV.get().expect("Env must be initialized before access")
}

/// Initializes the global environment configuration from the `.env` file and environment variables.
/// Panics if the `TOKEN` variable is not set.
pub fn parse_env() {
    let token = std::env::var("TOKEN").expect("TOKEN must be set in .env");
    let weights = std::env::var("WEIGHTS").unwrap_or_else(|_| DEFAULT_WEIGHTS.to_string());
    let prefix = std::env::var("PREFIX").unwrap_or_else(|_| "!".to_string());
    let hosts = std::env::var("HOSTS")
        .unwrap_or_else(|_| String::new())
        .split(',')
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string)
        .collect();
    let dev_room = std::env::var("DEV_ROOM").ok();

    ENV.set(Env {
        token,
        weights,
        prefix,
        hosts,
        dev_room,
    })
    .ok();
}
