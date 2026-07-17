//! Info command group.

use crate::command::core::context::Context;
use rue_macro::command;

/// Bot health check
#[command]
pub async fn ping(ctx: &Context<'_>) -> anyhow::Result<()> {
    ctx.reply("pong").await?;
    Ok(())
}
