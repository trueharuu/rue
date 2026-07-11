//! Traits and metadata for commands.

use async_trait::async_trait;
use super::context::Context;

/// Static metadata describing a command.
pub struct CommandMetadata {
    /// The primary name used to invoke this command.
    pub name: &'static str,
    /// Alternative names that also trigger this command.
    pub aliases: &'static [&'static str],
    /// A short description shown in help text.
    pub description: &'static str,
    /// Auto-generated usage string.
    pub usage: &'static str,
}

/// A registered chat command.
#[async_trait]
pub trait Command: Send + Sync {
    /// Returns the static metadata for this command.
    fn metadata(&self) -> &'static CommandMetadata;

    /// Execute this command with the given context.
    async fn execute(&self, ctx: &mut Context<'_>) -> anyhow::Result<()>;
}
