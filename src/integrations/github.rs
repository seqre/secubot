use anyhow::Result;

/// Minimal GitHub config used by the mirror helpers.
#[derive(Clone, Debug)]
pub struct GhCfg {
    pub repo: String,                 // "owner/repo"
    pub token: String,                // PAT / App token
    pub default_labels: Vec<String>,  // e.g., ["todo","triage"]
}

/// Create an issue and return its HTML URL.
pub async fn create_issue(cfg: &GhCfg, title: &str, body_md: &str) -> Result<String> {
    let (owner, repo) = cfg
        .repo
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("Invalid repo '{}', expected owner/repo", cfg.repo))?;

    let octo = octocrab::OctocrabBuilder::default()
        .personal_token(cfg.token.clone())
        .build()?;

    let issue = octo
        .issues(owner, repo)
        .create(title)
        .body(body_md.to_string())
        .labels(cfg.default_labels.clone())
        .send()
        .await?;

    Ok(issue.html_url.to_string())
}

