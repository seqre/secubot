#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::sync::{Arc, Mutex};

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
    settings: Settings,
}

impl Handler {
    pub fn new(secubot: Secubot, settings: Settings) -> Self {
        let commands = Commands::new(&secubot);
        Self {
            secubot,
            commands,
            settings,
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

        println!("Setting up guild slash commands:");
        for guild in &self.settings.commands.guilds {
            let guild_commands =
                GuildId::set_application_commands(&GuildId(guild.id), &ctx.http, |commands| {
                    self.commands.register_commands(commands, &guild.commands);
                    commands
                })
                .await;

            println!(
                " - Guild ({}) commands: {:?}",
                guild.id,
                guild_commands
                    .unwrap()
                    .iter()
                    .map(|c| String::from(&c.name))
                    .collect::<Vec<String>>()
            );
        }

        let global_commands =
            ApplicationCommand::set_global_application_commands(&ctx.http, |commands| {
                self.commands
                    .register_commands(commands, &self.settings.commands.globals);
                commands
            })
            .await;

        println!(
            "Setting up global slash commands: {:?}",
            global_commands
                .unwrap()
                .iter()
                .map(|c| String::from(&c.name))
                .collect::<Vec<String>>()
        );
    }
}

#[tokio::main]
async fn main() {
    let settings = Settings::new().unwrap();
    println!("{:#?}", settings);

    let database = SqliteConnection::establish(&settings.database.url)
        .expect(&format!("Error connecting to {}", &settings.database.url));

    embedded_migrations::run(&database);

    let token = String::from(&settings.discord_token);
    let application_id = settings.application_id;

    let conn = Arc::new(Mutex::new(database));
    let secubot = Secubot::new(conn);
    let handler = Handler::new(secubot, settings);
    let intents = GatewayIntents::non_privileged();

    let mut client = Client::builder(token, intents)
        .event_handler(handler)
        .application_id(application_id)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
