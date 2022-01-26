use std::env;

use serenity::{
    async_trait,
    model::{
        gateway::Ready,
        id::GuildId,
        interactions::{
            application_command::{
                ApplicationCommand,
                ApplicationCommandOptionType,
            },
            Interaction,
            InteractionResponseType,
        },
    },
    prelude::*,
};
use dotenv::dotenv;

use crate::commands::Command;

mod commands;

use commands::TodoActions;

pub struct Handler {
    db: sqlx::SqlitePool,
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let content = match command.data.name.as_str() {
                "ping" => commands::Ping::execute(&self, &command),
                "todo" => commands::Todo::execute(&self, &command),
                _ => "Command not implemented :(".to_string(),
            };

            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(content))
                })
            .await
            {
                println!("Cannot respond to slash command: {}", why);
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let guild_id = GuildId(
            env::var("GUILD_ID")
            .expect("Expected GUILD_ID in environment")
            .parse()
            .expect("GUILD_ID must be an integer"),
        );

        let commands = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            commands.create_application_command(|command| {
                command
                    .name("ping")
                    .description("A ping command")
            })
            .create_application_command(|command| {
                command
                    .name("todo")
                    .description("A todo")
                    .create_option(|option| {
                        option
                            .name("list")
                            .description("list todos")
                            .kind(ApplicationCommandOptionType::SubCommand)
                    })
                    .create_option(|option| {
                        option
                            .name("add")
                            .description("add todo")
                            .kind(ApplicationCommandOptionType::SubCommand)
                            .create_sub_option(|subopt| {
                                subopt
                                    .name("content")
                                    .description("todo content")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(true)
                            })
                    })
                    .create_option(|option| {
                        option
                            .name("delete")
                            .description("delete todo")
                            .kind(ApplicationCommandOptionType::SubCommand)
                            .create_sub_option(|subopt| {
                                subopt
                                    .name("id")
                                    .description("todo id")
                                    .kind(ApplicationCommandOptionType::Integer)
                                    .required(true)
                            })
                    })
                    .create_option(|option| {
                        option
                            .name("complete")
                            .description("complete todo")
                            .kind(ApplicationCommandOptionType::SubCommand)
                            .create_sub_option(|subopt| {
                                subopt
                                    .name("id")
                                    .description("todo id")
                                    .kind(ApplicationCommandOptionType::Integer)
                                    .required(true)
                            })
                    })
            })
        })
        .await;

        println!("I now have the following guild slash commands: {:#?}", commands);
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let application_id: u64 = env::var("APPLICATION_ID")
        .expect("Expected an application id in the environment")
        .parse()
        .expect("application id is not a valid id");

    use std::str::FromStr;
    let database = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(sqlx::sqlite::SqliteConnectOptions::from_str(
                &env::var("DATABASE_URL").expect("Expected a database url in the environment")
        )
            .expect("Incorrect database url")
            .create_if_missing(true),
        )
        .await
        .expect("Couldn't connect to database");

    let mut client = Client::builder(token)
        .event_handler(Handler { db: database })
        .application_id(application_id)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
