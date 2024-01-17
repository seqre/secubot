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

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Database {
    pub url: String,
}

impl Default for Database {
    fn default() -> Self {
        Self {
            url: "db.sqlite".to_string(),
        }
    }
}

//#[derive(Debug, Deserialize, Clone)]
//#[allow(unused)]
// pub struct Commands {
//    pub globals: Vec<String>,
//    pub guilds: Vec<Guild>,
//}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct BotSettings {
    #[serde(default = "Feature::all")]
    pub features: HashSet<Feature>,
}

impl Default for BotSettings {
    fn default() -> Self {
        Self {
            features: Feature::all(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Settings {
    // pub log_level: String,
    pub discord_token: String,
    // pub application_id: u64,
    #[serde(default)]
    pub database: Database,
    #[serde(default)]
    pub global: BotSettings,
    #[serde(default)]
    pub guilds: HashMap<GuildId, BotSettings>,
    // pub commands: Commands,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let cwd = match env::current_dir() {
            Ok(cwd) => cwd.display().to_string(),
            Err(_) => ".".to_string(),
        };
        // let mode = env::var("SCBT_RUN_MODE").unwrap_or_else(|_| "dev".into());

        debug!(
            "Looking for configuration file {cwd}/config and/or configuration files in {cwd}{}",
            "/config/"
        );

        let config = Config::builder()
            .add_source(File::with_name(&format!("{cwd}/config")).required(false))
            // .add_source(File::with_name(&format!("{cwd}/{mode}")).required(false))
            // .add_source(File::with_name(&format!("{prefix}/commands")))
            .add_source(
                glob(&format!("{cwd}/config/*"))
                    .unwrap()
                    .map(|path| File::from(path.unwrap()))
                    .collect::<Vec<_>>(),
            )
            .add_source(Environment::with_prefix("SCBT").separator("__"))
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
