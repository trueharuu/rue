//! Argument parsing for commands.
//!
//! The [`ParseArgument`] trait converts a parsed type from a command's argument
//! stream. A command with a type that implements this trait parses its
//! arguments automatically.
//!
//! This module supplies [`ParseArgument`] implementations for common types:
//! [`String`], [`i64`], [`u64`], [`f64`], and [`bool`].
use super::context::Context;

/// Trait for types that can be extracted from a command's argument stream.
pub trait ParseArgument: Sized {
    /// Attempt to parse one value of this type from the context.
    fn parse(ctx: &mut Context<'_>) -> anyhow::Result<Self>;

    /// Return a human-readable label for this argument type.
    fn label() -> String;
}

/// Consume the next word as a [`String`].
impl ParseArgument for String {
    fn parse(ctx: &mut Context<'_>) -> anyhow::Result<Self> {
        ctx.next_word()
            .ok_or_else(|| anyhow::anyhow!("missing argument"))
            .map(String::from)
    }

    fn label() -> String {
        "<text>".into()
    }
}

/// Consume the next word as an [`i64`].
impl ParseArgument for i64 {
    fn parse(ctx: &mut Context<'_>) -> anyhow::Result<Self> {
        let word = ctx
            .next_word()
            .ok_or_else(|| anyhow::anyhow!("missing argument"))?;
        word.parse()
            .map_err(|e| anyhow::anyhow!("invalid integer: {e}"))
    }

    fn label() -> String {
        "int".into()
    }
}

/// Consume the next word as a [`u64`].
impl ParseArgument for u64 {
    fn parse(ctx: &mut Context<'_>) -> anyhow::Result<Self> {
        let word = ctx
            .next_word()
            .ok_or_else(|| anyhow::anyhow!("missing argument"))?;
        word.parse()
            .map_err(|e| anyhow::anyhow!("invalid unsigned integer: {e}"))
    }

    fn label() -> String {
        "uint".into()
    }
}

/// Consume the next word as an [`f64`].
impl ParseArgument for f64 {
    fn parse(ctx: &mut Context<'_>) -> anyhow::Result<Self> {
        let word = ctx
            .next_word()
            .ok_or_else(|| anyhow::anyhow!("missing argument"))?;
        word.parse()
            .map_err(|e| anyhow::anyhow!("invalid float: {e}"))
    }

    fn label() -> String {
        "float".into()
    }
}

/// Consume the next word as a [`bool`].
///
/// Recognises `true`/`1`/`yes`/`on` as true and
/// `false`/`0`/`no`/`off` as false (case-insensitive).
impl ParseArgument for bool {
    fn parse(ctx: &mut Context<'_>) -> anyhow::Result<Self> {
        let word = ctx
            .next_word()
            .ok_or_else(|| anyhow::anyhow!("missing argument"))?;
        match word.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Ok(true),
            "false" | "0" | "no" | "off" => Ok(false),
            other => Err(anyhow::anyhow!("invalid boolean: {other}")),
        }
    }

    fn label() -> String {
        "bool".into()
    }
}

/// A newtype that captures all remaining argument text.
pub struct Rest(pub String);

impl ParseArgument for Rest {
    fn parse(ctx: &mut Context<'_>) -> anyhow::Result<Self> {
        Ok(Rest(ctx.rest().to_string()))
    }

    fn label() -> String {
        "<text>".into()
    }
}

/// Optionally parse a value; returns `None` on failure without
/// consuming any arguments.
impl<T: ParseArgument> ParseArgument for Option<T> {
    fn parse(ctx: &mut Context<'_>) -> anyhow::Result<Self> {
        let saved = ctx.save_position();
        if let Ok(val) = T::parse(ctx) {
            Ok(Some(val))
        } else {
            ctx.restore_position(saved);
            Ok(None)
        }
    }

    fn label() -> String {
        format!("[{}]", T::label())
    }
}
