//! Info command group.

use std::collections::BTreeMap;

use crate::command::context::Context;
use crate::command::traits::Category;
use crate::command::traits::Restriction;
use rue_macro::command;
use std::fmt::Write as _;

/// Bot health check
#[command(category = Category::Info)]
pub async fn ping(ctx: &Context<'_>) -> anyhow::Result<()> {
    ctx.reply("pong").await?;
    Ok(())
}

/// Returns the string name of a command category.
fn category_name(c: Category) -> &'static str {
    match c {
        Category::Info => "info",
        Category::Controls => "controls",
        Category::Solver => "solver",
        Category::Dev => "dev",
    }
}

/// Lists available commands, or gives detail on a single command.
#[command(aliases = ["h"], category = Category::Info)]
pub async fn help(ctx: &mut Context<'_>, name: Option<String>) -> anyhow::Result<()> {
    let show_dev = ctx.user.level == Restriction::Dev;

    if let Some(name) = name {
        let Some(cmd) = ctx.bot.registry.find(&name) else {
            ctx.reply("Command not found").await?;
            return Ok(());
        };
        let meta = cmd.metadata();
        if meta.category == Category::Dev && !show_dev {
            ctx.reply("Command not found").await?;
            return Ok(());
        }
        let alts = if meta.aliases.is_empty() {
            String::new()
        } else {
            format!(" ({})", meta.aliases.join("/"))
        };
        let usage = if meta.usage.is_empty() {
            String::new()
        } else {
            format!(" {}", meta.usage)
        };
        ctx.reply(&format!(
            "{}{}{}: {}",
            meta.name, alts, usage, meta.description
        ))
        .await?;
        return Ok(());
    }

    let mut by_category: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for cmd in ctx.bot.registry.iter() {
        let meta = cmd.metadata();
        if meta.category == Category::Dev && !show_dev {
            continue;
        }
        let entry = if meta.aliases.is_empty() {
            meta.name.to_string()
        } else {
            format!("{}/{}", meta.name, meta.aliases.join("/"))
        };
        by_category
            .entry(category_name(meta.category))
            .or_default()
            .push(entry);
    }

    let mut out = String::from("Available commands:");
    for (category, mut commands) in by_category {
        commands.sort();
        let _ = write!(
            out,
            "\n{}{}:\n  {}",
            category[..1].to_uppercase(),
            &category[1..],
            commands.join(", ")
        );
    }

    ctx.reply(&out).await?;
    Ok(())
}
