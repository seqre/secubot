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
        .into_iter()
        .map(|x| {
            if x.starts_with('#') {
                format!("**{}**\n", x)
            } else {
                format!("{}\n", x)
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
