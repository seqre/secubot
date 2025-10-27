#![feature(int_roundings)]
#![feature(lazy_cell)]

use std::sync::Arc;

use diesel::{
    r2d2::{ConnectionManager, Pool},
    sqlite::SqliteConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use poise::serenity_prelude as serenity;
use tracing::{debug, error, info};

use crate::{
    commands::{changelog, hall_of_fame, ping, todo},
    ctx_data::CtxData,
    settings::Settings,
};

// TODO:
// mod events;

#[allow(clippy::module_name_repetitions)]
mod commands;
mod integrations;

mod ctx_data;
mod framework;
mod models;
mod schema;
mod settings;

#[allow(clippy::module_name_repetitions)]
mod tasks;
mod utils;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/sqlite");
const VERSION: &str = env!("CARGO_PKG_VERSION");

type Result<T> = anyhow::Result<T>;
type Error = anyhow::Error;
type Context<'a> = poise::Context<'a, Arc<CtxData>, Error>;
type Conn = Pool<ConnectionManager<SqliteConnection>>;

fn setup_db(db_url: &String) -> Result<Conn> {
    let conn_man = ConnectionManager::<SqliteConnection>::new(db_url);
    let pool =
        Pool::new(conn_man).unwrap_or_else(|_| panic!("Error creating pool for: {}", &db_url));

    debug!("Running database migrations");
    match &pool.get()?.run_pending_migrations(MIGRATIONS) {
        Ok(_) => info!("Database migrations completed"),
        Err(e) => error!("Database migrations error: {:?}", e),
    };

    Ok(pool)
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
    tracing_subscriber::fmt()
        // TODO: dirty way, find better solution
        .with_env_filter("secubot=debug")
        .init();

    info!("Running v{VERSION}");

    let settings = Settings::new().expect("Missing configuration");

    let mut clean_settings = settings.clone();
    clean_settings.discord_token = String::from("<REDACTED>");
    info!("Parsed configuration: {:?}", &clean_settings);

    let conn = setup_db(&settings.database.url).expect("Couldn't initialize database connection");
    let ctx_data = Arc::new(CtxData::new(conn, clean_settings));

    let options = poise::FrameworkOptions {
        commands: vec![
            commands::help(),
            commands::register(),
            changelog::changelog(),
            changelog::version(),
            ping::ping(),
            todo::todo(),
            commands::gh::gh(),
            hall_of_fame::hof(),
        ],
        event_handler: |ctx, event, framework, data| {
            Box::pin(framework::event_handler(ctx, event, framework, data))
        },
        on_error: |error| Box::pin(framework::on_error(error)),
        ..Default::default()
    };

    poise::Framework::builder()
        .token(&settings.discord_token)
        .options(options)
        .intents(get_intents())
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                // Auto-register slash commands on startup.
                // Set SCBT__REGISTER_GLOBAL=1 to register globally; otherwise per-guild.
                use poise::builtins::{register_globally, register_in_guild};

                let register_global =
                    std::env::var("SCBT__REGISTER_GLOBAL").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false);

                if register_global {
                    if let Err(e) = register_globally(ctx, &framework.options().commands).await {
                        tracing::warn!("Global command registration failed: {:?}", e);
                    } else {
                        tracing::info!("Global commands registered");
                    }
                } else {
                    // Register for each guild the bot is currently in (instant in that guild)
                    for g in &ready.guilds {
                        if let Err(e) = register_in_guild(ctx, &framework.options().commands, g.id).await {
                            tracing::warn!("Guild command registration failed for {}: {:?}", g.id, e);
                        } else {
                            tracing::info!("Commands registered in guild {}", g.id);
                        }
                    }
                }

                framework::setup(ctx, ready, framework, ctx_data).await
            })
        })
        .run()
        .await
        .expect("Couldn't initialize bot")
}

