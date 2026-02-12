use std::{
    collections::{HashMap, HashSet},
    env,
    hash::Hash,
};

use config::{Config, ConfigError, Environment, File};
use glob::glob;
use poise::serenity_prelude::{CacheHttp, Channel, ChannelId, GuildId};
use serde_derive::Deserialize;
use tracing::debug;

/* ─────────────────────────── Features ─────────────────────────── */

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Feature {
    NotifyOnDeletedMessages,
    PeriodicTodoReminders,
}

impl Feature {
    fn all() -> HashSet<Self> {
        HashSet::from([
            Feature::NotifyOnDeletedMessages,
            Feature::PeriodicTodoReminders,
        ])
    }
}

/* ─────────────────────────── Database ─────────────────────────── */

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Database {
    pub url: String,
}

impl Default for Database {
    fn default() -> Self {
        Self { url: "db.sqlite".to_string() }
    }
}

/* ─────────────────────────── Bot settings ─────────────────────────── */

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct BotSettings {
    #[serde(default = "Feature::all")]
    pub features: HashSet<Feature>,
}

impl Default for BotSettings {
    fn default() -> Self {
        Self { features: Feature::all() }
    }
}

/* ─────────────────────────── GitHub settings ─────────────────────────── */

fn default_github_labels() -> Vec<String> {
    vec!["todo".into(), "triage".into()]
}

#[derive(Debug, Deserialize, Clone)]
pub struct GithubSettings {
    /// Default repository in "owner/repo" form, used when no channel override exists.
    #[serde(default)]
    pub repo: String,
    /// PAT or GitHub App installation token (read/write issues at minimum).
    #[serde(default)]
    pub token: String,
    /// Default labels to apply to created issues.
    #[serde(default = "default_github_labels")]
    pub default_labels: Vec<String>,
    /// If non-empty, only these channel IDs are allowed to mirror to GitHub.
    #[serde(default)]
    pub allowed_channels: HashSet<u64>,
    /// Channel-specific repo mapping: channel_id -> "owner/repo".
    #[serde(default)]
    pub channel_map: HashMap<u64, String>,
}

impl Default for GithubSettings {
    fn default() -> Self {
        Self {
            repo: String::new(),
            token: String::new(),
            default_labels: default_github_labels(),
            allowed_channels: HashSet::new(),
            channel_map: HashMap::new(),
        }
    }
}

/* ─────────────────────────── Root Settings ─────────────────────────── */

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Settings {
    // pub log_level: String,
    pub discord_token: String,
    #[serde(default)]
    pub database: Database,
    #[serde(default)]
    pub global: BotSettings,
    #[serde(default)]
    pub guilds: HashMap<GuildId, BotSettings>,

    /// GitHub integration & routing config.
    #[serde(default)]
    pub github: GithubSettings,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let cwd = match env::current_dir() {
            Ok(cwd) => cwd.display().to_string(),
            Err(_) => ".".to_string(),
        };

        debug!(
            "Looking for configuration file {cwd}/config and/or configuration files in {cwd}{}",
            "/config/"
        );

        // Load: base file `./config` (any extension), all `./config/*`, then env.
        // ENV mapping: SCBT__FOO__BAR -> foo.bar ; lists split by comma.
        let config = Config::builder()
            .add_source(File::with_name(&format!("{cwd}/config")).required(false))
            .add_source(
                glob(&format!("{cwd}/config/*"))
                    .unwrap()
                    .map(|path| File::from(path.unwrap()))
                    .collect::<Vec<_>>(),
            )
            .add_source(
                Environment::with_prefix("SCBT")
                    .separator("__")
                    .list_separator(","), // e.g. SCBT__GITHUB__DEFAULT_LABELS=todo,triage
            )
            .build()?;

        config.try_deserialize()
    }

    pub async fn is_feature_enabled(
        &self,
        feature: &Feature,
        cache_http: impl CacheHttp,
        channel_id: &ChannelId,
    ) -> bool {
        let channel = channel_id.to_channel(&cache_http).await.ok();
        let default = self.global.features.contains(feature);

        if let Some(Channel::Guild(guild)) = channel {
            self.guilds
                .get(&guild.guild_id)
                .map_or(default, |guild| guild.features.contains(feature))
        } else {
            default
        }
    }
}

