use octocrab::Octocrab;

use crate::{Context, Result};

/// Get the latest changelog
#[poise::command(slash_command)]
pub async fn changelog(ctx: Context<'_>) -> Result<()> {
    let octocrab = Octocrab::builder().build()?;
    let release = &octocrab
        .repos("seqre", "secubot")
        .releases()
        .get_latest()
        .await?;

    let changelog: String = release
        .body
        .as_ref()
        .unwrap()
        .split("\r\n")
        .map(|x| {
            if x.starts_with('#') {
                format!("**{x}**\n")
            } else {
                format!("{x}\n")
            }
        })
        .collect();

    let changelog = if changelog.is_empty() {
        "No changelog in the release notes."
    } else {
        &changelog
    };

    ctx.say(changelog).await?;

    Ok(())
}
/// Get running version of the bot
#[poise::command(slash_command)]
pub async fn version(ctx: Context<'_>) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");

    ctx.say(format!("v{version}")).await?;

    Ok(())
}
