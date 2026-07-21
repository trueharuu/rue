//! Info command group.

use rue_macro::command;
use crate::command::core::context::Context;
use super::core::traits::Category;


/// Bot health check
#[command(category = Category::Info)]
pub async fn ping(ctx: &Context<'_>) -> anyhow::Result<()> {
    ctx.reply("pong").await?;
    Ok(())
}
