use config::{Config, ConfigError, Environment, File};
use glob::glob;
use serde_derive::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Guild {
    pub id: u64,
    pub commands: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Commands {
    pub globals: Vec<String>,
    pub guilds: Vec<Guild>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Settings {
    pub log_level: String,
    pub discord_token: String,
    pub application_id: u64,
    pub database: Database,
    pub commands: Commands,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mode = env::var("SCBT_RUN_MODE").unwrap_or_else(|_| "dev".into());
        let config = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", mode)))
            .add_source(File::with_name("config/commands"))
            .add_source(
                glob("config/custom/*")
                    .unwrap()
                    .map(|path| File::from(path.unwrap()))
                    .collect::<Vec<_>>(),
            )
            .add_source(Environment::with_prefix("SCBT").separator("__"))
            .build()?;

        config.try_deserialize()
    }
}
