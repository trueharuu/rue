//! Controls command group.

use rue_macro::command;
use triangle::types::room::Bracket;

use crate::bot::state::Finesse;
use crate::command::context::Context;
use crate::command::traits::Category;
use crate::command::traits::Restriction;
use crate::utils::FAILURE;
use crate::utils::SUCCESS;
use crate::utils::WARNING;

/// Absolute minimum pieces per second.
const PPS_MIN: f64 = 0.5;
/// Absolute maximum pieces per second.
const PPS_MAX: f64 = 10.0;
/// Maximum pieces per second for smooth finesse.
const PPS_SMOOTH_MAX: f64 = 5.0;

/// Kills the bot (from the room)
#[command(category = Category::Controls, restriction_level = Restriction::Host)]
pub async fn kill(ctx: &Context<'_>) -> anyhow::Result<()> {
    ctx.reply("bye :oyes:/").await?;
    ctx.bot.destroy().await;
    Ok(())
}

/// Move the bot to the player's bracket
#[command(aliases = ["e"], category = Category::Controls, restriction_level = Restriction::Host)]
pub async fn enable(ctx: &Context<'_>, force: Option<String>) -> anyhow::Result<()> {
    let enabled = ctx.bot.state.read().await.enabled.value;
    if enabled {
        ctx.reply(&format!("{WARNING} already enabled")).await?;
        return Ok(());
    }

    let force = force.as_deref() == Some("force") && ctx.user.level == Restriction::Dev;

    let Some(mut room) = ctx.bot.client.room() else {
        ctx.reply(&format!("{WARNING} not in a room")).await?;
        return Ok(());
    };

    match room.switch(Bracket::Player).await {
        Ok(()) => {
            {
                let mut state = ctx.bot.state.write().await;
                state.enabled.value = true;
                state.enabled.attempt = false;
                state.enabled.force = force;
            }

            ctx.reply(SUCCESS).await?;
        }
        Err(e) => {
            ctx.reply(&format!("{FAILURE} error switching bracket: {e}"))
                .await?;
        }
    }

    Ok(())
}

/// Move the bot to the spectators bracket
#[command(aliases = ["d"], category = Category::Controls, restriction_level = Restriction::Host)]
pub async fn disable(ctx: &Context<'_>) -> anyhow::Result<()> {
    {
        let mut state = ctx.bot.state.write().await;
        state.enabled.attempt = false;
        state.enabled.force = false;
    }

    let enabled = ctx.bot.state.read().await.enabled.value;
    if !enabled {
        ctx.reply(&format!("{FAILURE} already disabled")).await?;
        return Ok(());
    }

    if let Some(mut room) = ctx.bot.client.room() {
        ctx.bot.state.write().await.enabled.value = false;
        match room.switch(Bracket::Spectator).await {
            Ok(()) => ctx.reply(SUCCESS).await?,
            Err(_) => {
                ctx.reply(&format!("{FAILURE} error disabling gameplay"))
                    .await?;
            }
        }
    } else {
        ctx.bot.state.write().await.enabled.value = false;
        ctx.reply(SUCCESS).await?;
    }

    Ok(())
}

/// Restricts the bot to a certain level
#[command(category = Category::Controls, restriction_level = Restriction::Host)]
pub async fn restrict(ctx: &Context<'_>, level: String) -> anyhow::Result<()> {
    let v = match level.as_str() {
        "none" => Restriction::None,
        "player" => Restriction::Player,
        "host" => Restriction::Host,
        "dev" => Restriction::Dev,
        _ => {
            ctx.reply(&format!("{FAILURE} invalid restriction level `{level}`"))
                .await?;
            return Ok(());
        }
    };

    if ctx.user.level == Restriction::Player || ctx.user.level == Restriction::None {
        ctx.reply(&format!(
            "{FAILURE} players cannot change restriction levels"
        ))
        .await?;
        return Ok(());
    }

    if v == Restriction::Dev && ctx.user.level != Restriction::Dev {
        ctx.reply(&format!(
            "{FAILURE} this restriction level is locked to developers"
        ))
        .await?;
        return Ok(());
    }

    ctx.bot.state.write().await.restriction = v;

    ctx.reply(SUCCESS).await?;

    Ok(())
}

/// Set the bot's pps
#[command(aliases = ["p"], category = Category::Controls, restriction_level = Restriction::Host)]
pub async fn pps(ctx: &Context<'_>, speed: Option<f64>) -> anyhow::Result<()> {
    let Some(speed) = speed else {
        let current = ctx.bot.config.read().await.pps;
        ctx.reply(&format!("{current}")).await?;
        return Ok(());
    };

    if speed.is_nan() {
        ctx.reply(&format!("{FAILURE} invalid pps (not a number)"))
            .await?;
        return Ok(());
    }

    let bypass = ctx.user.level == Restriction::Dev;
    if speed < PPS_MIN && !bypass {
        ctx.reply(&format!("{FAILURE} invalid pps (less than {PPS_MIN})"))
            .await?;
        return Ok(());
    }
    if speed > PPS_MAX && !bypass {
        ctx.reply(&format!("{FAILURE} invalid pps (greater than {PPS_MAX})"))
            .await?;
        return Ok(());
    }

    let finesse = ctx.bot.config.read().await.finesse;
    if speed > PPS_SMOOTH_MAX && finesse == Finesse::Smooth {
        ctx.bot.config.write().await.finesse = Finesse::Instant;
        ctx.reply(&format!(
            "{WARNING} instant finesse enabled for pps above {PPS_SMOOTH_MAX}"
        ))
        .await?;
    }

    let rounded = (speed * 1000.0).round() / 1000.0;
    ctx.bot.config.write().await.pps = rounded;
    ctx.reply(SUCCESS).await?;

    Ok(())
}

/// Toggle the bot's burst mode
#[command(aliases = ["b"], category = Category::Controls, restriction_level = Restriction::Host)]
pub async fn burst(ctx: &Context<'_>, value: Option<String>) -> anyhow::Result<()> {
    let Some(value) = value else {
        let current = ctx.bot.config.read().await.burst;
        ctx.reply(if current { "on" } else { "off" }).await?;
        return Ok(());
    };

    let value = match value.as_str() {
        "on" => true,
        "off" => false,
        _ => {
            ctx.reply(&format!(
                "{FAILURE} invalid burst value (must be 'on' or 'off')"
            ))
            .await?;
            return Ok(());
        }
    };

    ctx.bot.config.write().await.burst = value;
    ctx.reply(SUCCESS).await?;

    Ok(())
}

/// Set the bot's finesse mode
#[command(aliases = ["f"], category = Category::Controls, restriction_level = Restriction::Host)]
pub async fn finesse(ctx: &Context<'_>, mode: Option<String>) -> anyhow::Result<()> {
    let Some(mode) = mode else {
        let current = ctx.bot.config.read().await.finesse;
        let name = match current {
            Finesse::Smooth => "smooth",
            Finesse::Instant => "instant",
        };
        ctx.reply(name).await?;
        return Ok(());
    };

    let parsed = match mode.as_str() {
        "smooth" => Finesse::Smooth,
        "instant" => Finesse::Instant,
        _ => {
            ctx.reply(&format!(
                "{FAILURE} invalid finesse mode (must be 'smooth' or 'instant')"
            ))
            .await?;
            return Ok(());
        }
    };

    if parsed == Finesse::Smooth {
        let speed = ctx.bot.config.read().await.pps;
        if speed > PPS_SMOOTH_MAX {
            ctx.reply(&format!(
                "{FAILURE} smooth finesse is not allowed for pps above {PPS_SMOOTH_MAX}"
            ))
            .await?;
            return Ok(());
        }
    }

    ctx.bot.config.write().await.finesse = parsed;
    ctx.reply(SUCCESS).await?;

    Ok(())
}
