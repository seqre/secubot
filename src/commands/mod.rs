use crate::{Context, Result};

pub mod changelog;
pub mod ping;

#[allow(clippy::cast_precision_loss)]
#[allow(clippy::cast_sign_loss)]
pub mod todo;

pub const DISCORD_EMBED_FIELDS_LIMIT: u32 = 25;

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
