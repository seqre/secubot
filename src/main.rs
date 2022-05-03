#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dotenv::dotenv;
use std::{
    env,
    sync::{Arc, Mutex},
};

use serenity::{
    async_trait,
    model::{
        gateway::Ready,
        id::GuildId,
        interactions::{application_command::ApplicationCommand, Interaction},
    },
    prelude::*,
};

use crate::commands::Commands;
use crate::secubot::Secubot;

mod commands;
mod models;
mod schema;
mod secubot;

embed_migrations!();

struct Handler {
    secubot: Secubot,
    commands: Commands,
}

impl Handler {
    pub fn new(secubot: Secubot) -> Self {
        Self {
            secubot,
            commands: Commands::new(),
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        self.commands.handle(ctx, interaction, &self.secubot).await;
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        //let guild_id = GuildId(
        //    env::var("GUILD_ID")
        //        .expect("Expected GUILD_ID in environment")
        //        .parse()
        //        .expect("GUILD_ID must be an integer"),
        //);

        //let guild_commands = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
        //    self.commands.register_commands(commands);
        //    commands
        //})
        //.await;

        //println!(
        //    "I now have the following guild slash commands: {:#?}",
        //    guild_commands
        //);

        let global_commands =
            ApplicationCommand::set_global_application_commands(&ctx.http, |commands| {
                self.commands.register_commands(commands);
                commands
            })
                .await;

        println!(
            "I created the following global slash command: {:#?}",
            global_commands
        );
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let token = env::var("DISCORD_TOKEN").expect("Expected a DISCORD_TOKEN in the environment");

    let application_id: u64 = env::var("APPLICATION_ID")
        .expect("Expected an APPLICATION_ID in the environment")
        .parse()
        .expect("APPLICATION_ID is not a valid id");

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let database =
        SqliteConnection::establish(&db_url).expect(&format!("Error connecting to {}", db_url));

    embedded_migrations::run(&database);

    let conn = Arc::new(Mutex::new(database));
    let secubot = Secubot::new(conn);
    let handler = Handler::new(secubot);
    let intents = GatewayIntents::non_privileged();
        //| GatewayIntents::GUILD_MEMBERS
        //| GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(token, intents)
        .event_handler(handler)
        .application_id(application_id)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
