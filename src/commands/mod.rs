use std::sync::{LazyLock};

use regex::Regex;
use time::{format_description, format_description::FormatItem};

use crate::{Context, Result};

pub mod changelog;
pub mod hall_of_fame;
pub mod ping;

#[allow(clippy::cast_precision_loss)]
#[allow(clippy::cast_sign_loss)]
pub mod todo;

pub const DISCORD_EMBED_FIELDS_LIMIT: u32 = 24;

static USER_PING_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<@(\d+)>").unwrap());
static TIME_FORMAT: LazyLock<Vec<FormatItem<'static>>> = LazyLock::new(|| {
    format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap()
});

#[poise::command(track_edits, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<()> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "Type /help command:<command> to get more info on a command.",
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command, hide_in_help, owners_only)]
pub async fn register(ctx: Context<'_>) -> Result<()> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}
