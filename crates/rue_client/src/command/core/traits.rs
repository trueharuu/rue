//! Traits and metadata for commands.

use super::context::Context;
use async_trait::async_trait;

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash, Default)]
pub enum Restriction {
    #[default]
    None,
    Player,
    Host,
    Dev,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum Category {
    #[default]
    Info,
    Controls,
    Solver,
    Dev,
}

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
    // todo: doc
    pub restriction_level: Restriction,
    // todo: doc
    pub category: Category,
}

/// A registered chat command.
#[async_trait]
pub trait Command: Send + Sync {
    /// Returns the static metadata for this command.
    fn metadata(&self) -> &'static CommandMetadata;

    /// Execute this command with the given context.
    async fn execute(&self, ctx: &mut Context<'_>) -> anyhow::Result<()>;
}
