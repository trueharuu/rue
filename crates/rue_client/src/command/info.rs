//! Info command group.

use rue_macro::command;
use crate::command::core::context::Context;


/// Bot health check
#[command]
pub async fn ping(ctx: &Context<'_>) -> anyhow::Result<()> {
    ctx.reply("pong").await?;
    Ok(())
}
