//! Environment configuration for the TETR.IO client.
use std::sync::OnceLock;

/// Global environment configuration for the TETR.IO client, including the API token and weights path.
pub struct Env {
    /// API token for the TETR.IO client, used for authentication with the TETR.IO API.
    pub token: String,
    /// Path to the weights file.
    pub weights: String,
}

/// Default weights path, resolved from the source tree at compile time so it
/// doesn't depend on the working directory the binary happens to be launched from.
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

    ENV.set(Env { token, weights }).ok();
}
