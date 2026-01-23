use tracing::{info, warn};

use crate::{
    Context,
    integrations::github::{GhCfg, create_issue},
};

pub async fn mirror_for_channel(
    ctx: Context<'_>,
    title: &str,
    body_extra_md: Option<&str>,
) -> anyhow::Result<Option<String>> {
    let settings = &ctx.data().settings;
    let ch = ctx.channel_id().0;

    if settings.github.token.is_empty() {
        info!("mirror: skipped (missing GitHub token)");
        return Ok(None);
    }

    // If a whitelist exists, require membership
    if !settings.github.allowed_channels.is_empty()
        && !settings.github.allowed_channels.contains(&ch)
    {
        info!("mirror: skipped (channel {} not in allowed_channels)", ch);
        return Ok(None);
    }

    // channel_map override > default repo
    let repo = settings
        .github
        .channel_map
        .get(&ch)
        .cloned()
        .unwrap_or_else(|| settings.github.repo.clone());

    if repo.is_empty() {
        info!("mirror: skipped (no repo resolved for channel {})", ch);
        return Ok(None);
    }

    let gh = GhCfg {
        repo,
        token: settings.github.token.clone(),
        default_labels: settings.github.default_labels.clone(),
    };

    // Body shaped to satisfy your workflow validator (needs bullet lines)
    let body = format!(
        "### Context\nSubmitted via Discord by @{} in <#{}>.\n{}\n\n### Acceptance criteria\n- Clear user impact\n- Implementation approach agreed\n- Tests cover new behavior",
        ctx.author().name,
        ctx.channel_id(),
        body_extra_md.unwrap_or("")
    );

    match create_issue(&gh, title, &body).await {
        Ok(url) => {
            info!("mirror: created issue {url}");
            Ok(Some(url))
        }
        Err(e) => {
            warn!("mirror: GitHub issue creation failed: {e:?}");
            Err(e)
        }
    }
}

