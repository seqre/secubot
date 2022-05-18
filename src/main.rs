#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use env_logger;
use log::{error, info, warn, LevelFilter};
use std::{
    error::Error,
    str::FromStr,
    sync::{Arc, Mutex},
};

use serenity::prelude::*;

use crate::handler::Handler;
use crate::secubot::{Conn, Secubot};
use crate::settings::Settings;

mod commands;
mod handler;
mod models;
mod schema;
mod secubot;
mod settings;

embed_migrations!();

fn establish_db_conn(db_url: &String) -> Result<Conn, Box<dyn Error>> {
    let database =
        SqliteConnection::establish(&db_url).expect(&format!("Error connecting to {}", &db_url));

    match embedded_migrations::run(&database) {
        Ok(_) => info!("Database migrations completed"),
        Err(e) => error!("Database migrations error: {:?}", e),
    };

    Ok(Arc::new(Mutex::new(database)))
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

    let conn = establish_db_conn(&settings.database.url).expect("Error connecting to database");
    let secubot = Secubot::new(conn);
    let handler = Handler::new(secubot, settings);
    let intents = GatewayIntents::non_privileged();

    let mut client = Client::builder(token, intents)
        .event_handler(handler)
        .application_id(application_id)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
