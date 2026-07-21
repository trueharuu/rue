use std::sync::OnceLock;

pub struct Env {
  pub token: String,
  pub weights: String,
}

/// Default weights path, resolved from the source tree at compile time so it
/// doesn't depend on the working directory the binary happens to be launched from.
const DEFAULT_WEIGHTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../weights/simple.json");

static ENV: OnceLock<Env> = OnceLock::new();

pub fn env() -> &'static Env {
  ENV.get().expect("Env must be initialized before access")
}

pub fn parse_env() {
  let token = std::env::var("TOKEN").expect("TOKEN must be set in .env");
  let weights = std::env::var("WEIGHTS").unwrap_or_else(|_| DEFAULT_WEIGHTS.to_string());

  ENV.set(Env { token, weights }).ok();
}
