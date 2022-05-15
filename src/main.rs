#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
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
use crate::settings::Settings;

mod commands;
mod models;
mod schema;
mod secubot;
mod settings;

embed_migrations!();

struct Handler {
    secubot: Secubot,
    commands: Commands,
}

impl Handler {
    pub fn new(secubot: Secubot) -> Self {
        let commands = Commands::new(&secubot);
        Self { secubot, commands }
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
    let settings = Settings::new().unwrap();
    println!("{:#?}", settings);

    let database = SqliteConnection::establish(&settings.database.url)
        .expect(&format!("Error connecting to {}", settings.database.url));

    embedded_migrations::run(&database);

    let conn = Arc::new(Mutex::new(database));
    let secubot = Secubot::new(conn);
    let handler = Handler::new(secubot);
    let intents = GatewayIntents::non_privileged();

    let mut client = Client::builder(settings.discord_token, intents)
        .event_handler(handler)
        .application_id(settings.application_id)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
