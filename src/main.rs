use std::{error::Error, str::FromStr};

use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, Pool},
    sqlite::SqliteConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use log::{error, info, warn, LevelFilter};
use serenity::prelude::*;

use crate::{
    handler::Handler,
    secubot::{Conn, Secubot},
    settings::Settings,
};

mod commands;
mod events;
mod handler;
mod models;
mod schema;
mod secubot;
mod settings;
mod tasks;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/sqlite");

fn setup_db(db_url: &String) -> Result<Conn, Box<dyn Error>> {
    let conn_man = ConnectionManager::<SqliteConnection>::new(db_url);
    let pool =
        Pool::new(conn_man).unwrap_or_else(|_| panic!("Error creating pool for: {}", &db_url));

    match &pool.get().unwrap().run_pending_migrations(MIGRATIONS) {
        Ok(_) => info!("Database migrations completed"),
        Err(e) => error!("Database migrations error: {:?}", e),
    };

    Ok(pool)
}

fn get_intents() -> GatewayIntents {
    let mut base = GatewayIntents::non_privileged();
    if cfg!(feature = "msg_content") {
        base |= GatewayIntents::MESSAGE_CONTENT;
    }
    base
}

#[tokio::main]
async fn main() {
    let settings = Settings::new().unwrap();

    let log_level = LevelFilter::from_str(&settings.log_level).unwrap_or_else(|_| {
        warn!("Incorrect log_level in config, using Debug");
        LevelFilter::Debug
    });
    env_logger::Builder::new()
        .filter_module("secubot", log_level)
        .init();

    let mut clean_settings = settings.clone();
    clean_settings.discord_token = String::from("REDACTED");
    info!("Parsed configuration: {:?}", clean_settings);

    let token = String::from(&settings.discord_token);
    let application_id = settings.application_id;

    let conn = setup_db(&settings.database.url).expect("Error connecting to database");
    let secubot = Secubot::new(conn);
    let handler = Handler::new(secubot, settings);
    let intents = get_intents();

    let mut client = Client::builder(token, intents)
        .event_handler(handler)
        .application_id(application_id)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
