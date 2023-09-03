use std::env;

use config::{Config, ConfigError, Environment, File};
use glob::glob;
use serde_derive::Deserialize;
use tracing::debug;

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
    // pub log_level: String,
    pub discord_token: String,
    // pub application_id: u64,
    pub database: Database,
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
}
