use poise::serenity_prelude::Member;
use crate::{Context, Result};
use crate::integrations::mirror::mirror_for_channel;

/// GitHub utilities (channel-mapped)
#[poise::command(
    slash_command,
    subcommands("issue", "where_"), // <-- function names, not display names
)]
pub async fn gh(_ctx: Context<'_>) -> Result<()> {
    Ok(())
}

/// Create a GitHub Issue for this channel (uses allowed_channels + channel_map)
#[poise::command(slash_command)]
pub async fn issue(
    ctx: Context<'_>,
    #[description = "Short title"] title: String,
    #[description = "Details / context (optional)"] details: Option<String>,
    #[description = "Ping a teammate (optional)"] assignee: Option<Member>,
) -> Result<()> {
    let extra = match (&details, &assignee) {
        (Some(d), Some(a)) => format!("Assignee: @{}\n\n{}", a.user.name, d),
        (Some(d), None)    => d.clone(),
        (None,    Some(a)) => format!("Assignee: @{}", a.user.name),
        (None,    None)    => String::new(),
    };

    match mirror_for_channel(ctx, &title, Some(&extra)).await {
        Ok(Some(url)) => { ctx.say(format!("📬 Created: {url}")).await?; }
        Ok(None)      => { ctx.say("ℹ️ Mirroring is disabled for this channel.").await?; }
        Err(_)        => { ctx.say("⚠️ Failed to create GitHub issue.").await?; }
    }
    Ok(())
}

/// Show which repo this channel maps to (or if mirroring is disabled)
#[poise::command(slash_command, rename = "where")] // users type /gh where
pub async fn where_(ctx: Context<'_>) -> Result<()> {
    let settings = &ctx.data().settings;
    let ch = ctx.channel_id().0;

    if settings.github.token.is_empty() {
        ctx.say("GitHub mirroring is not configured (missing token).").await?;
        return Ok(());
    }

    if !settings.github.allowed_channels.is_empty()
        && !settings.github.allowed_channels.contains(&ch)
    {
        ctx.say(format!("This channel ({ch}) is **not** allowed to mirror.")).await?;
        return Ok(());
    }

    let repo = settings
        .github
        .channel_map
        .get(&ch)
        .cloned()
        .unwrap_or_else(|| settings.github.repo.clone());

    if repo.is_empty() {
        ctx.say(format!("No repository mapped for channel {ch}.")).await?;
    } else {
        ctx.say(format!("Channel {ch} → `{repo}`")).await?;
    }

    Ok(())
}

