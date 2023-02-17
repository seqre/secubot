#[allow(clippy::unused_self)]
use std::str::FromStr;

use diesel::{
    r2d2::{ConnectionManager, Pool},
    sqlite::SqliteConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use log::{debug, error, info, warn, LevelFilter};
use poise::serenity_prelude as serenity;

use crate::{
    commands::{changelog, ping, todo},
    ctx_data::CtxData,
    settings::Settings,
};

// mod events;
// mod handler;
mod commands;
mod ctx_data;
mod framework;
mod models;
mod schema;
mod settings;
// mod tasks;

type Result<T> = anyhow::Result<T>;
type Error = anyhow::Error;
type Context<'a> = poise::Context<'a, CtxData, anyhow::Error>;
type Conn = Pool<ConnectionManager<SqliteConnection>>;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/sqlite");

fn setup_db(db_url: &String) -> Conn {
    let conn_man = ConnectionManager::<SqliteConnection>::new(db_url);
    let pool =
        Pool::new(conn_man).unwrap_or_else(|_| panic!("Error creating pool for: {}", &db_url));

    match &pool.get().unwrap().run_pending_migrations(MIGRATIONS) {
        Ok(_) => info!("CtxDatabase migrations completed"),
        Err(e) => error!("CtxDatabase migrations error: {:?}", e),
    };

    pool
}

fn get_intents() -> serenity::GatewayIntents {
    let mut base = serenity::GatewayIntents::non_privileged();
    if cfg!(feature = "msg_content") {
        base |= serenity::GatewayIntents::MESSAGE_CONTENT;
    }
    base
}

#[tokio::main]
async fn main() {
    let settings = Settings::new().expect("Missing configurtaion!");

    let log_level = LevelFilter::from_str(&settings.log_level).unwrap_or_else(|_| {
        warn!("Incorrect log_level in config, using Debug");
        LevelFilter::Debug
    });
    env_logger::Builder::new()
        .filter_module("secubot", log_level)
        .init();

    let mut clean_settings = settings.clone();
    clean_settings.discord_token = String::from("<REDACTED>");
    info!("Parsed configuration: {:?}", clean_settings);

    let conn = setup_db(&settings.database.url);
    let ctx_data = CtxData::new(conn);

    let options = poise::FrameworkOptions {
        commands: vec![
            commands::help(),
            changelog::changelog(),
            ping::ping(),
            todo::todo(),
        ],
        event_handler: |_ctx, event, _framework, _data| {
            Box::pin(async move {
                debug!("Got an event in event handler: {:?}", event.name());
                Ok(())
            })
        },
        on_error: |error| Box::pin(framework::on_error(error)),
        ..Default::default()
    };

    poise::Framework::builder()
        .token(&settings.discord_token)
        .options(options)
        .intents(get_intents())
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                let empty: &[poise::structs::Command<CtxData, Error>] = &[];
                poise::builtins::register_globally(ctx, empty).await?;
                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    serenity::GuildId(settings.commands.guilds[0].id),
                )
                .await?;
                Ok(ctx_data)
            })
        })
        .run()
        .await
        .unwrap();
}
