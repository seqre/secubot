use async_trait::async_trait;
use log::{debug, warn};
use serenity::{
    builder::{CreateApplicationCommand, CreateApplicationCommands},
    client::Context,
    model::interactions::{
        application_command::ApplicationCommandInteraction, Interaction, InteractionResponseType,
    },
};
use std::{collections::HashMap, error::Error};

use crate::{
    commands::{ping::PingCommand, todo::TodoCommand},
    Secubot,
};

mod ping;
mod todo;

pub type CommandResult = Result<(), Box<dyn Error>>;

#[async_trait]
pub trait Command: Send + Sync {
    fn get_name(&self) -> &'static str;
    fn add_application_command(&self, command: &mut CreateApplicationCommand);
    async fn handle(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
        secubot: &Secubot,
    ) -> CommandResult;
}

pub struct Commands {
    commands: HashMap<String, Box<dyn Command>>,
}

impl Commands {
    pub fn new(secubot: &Secubot) -> Self {
        Self {
            commands: Self::get_commands(secubot),
        }
    }

    fn get_commands(secubot: &Secubot) -> HashMap<String, Box<dyn Command>> {
        let commands: Vec<Box<dyn Command>> = vec![
            Box::new(TodoCommand::new(secubot)),
            Box::new(PingCommand::new()),
        ];

        commands
            .into_iter()
            .map(|c| (c.get_name().into(), c))
            .collect()
    }

    pub fn register_commands(&self, creator: &mut CreateApplicationCommands, names: &Vec<String>) {
        for comm_name in names {
            if let Some(comm) = &self.commands.get(comm_name) {
                creator.create_application_command(|com| {
                    comm.add_application_command(com);
                    com.name(comm.get_name())
                });
            }
        }
    }

    pub async fn handle(&self, ctx: Context, interaction: Interaction, secubot: &Secubot) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let requested_comm = command.data.name.as_str();

            if let Some(bot_command) = &self.commands.get(requested_comm) {
                let error_message =
                    if let Err(e) = bot_command.handle(&ctx, &command, secubot).await {
                        warn!("Could not respond: {:?}", e);
                        format!("Could not generate response:\n```\n{}\n```", e)
                    } else {
                        String::from("")
                    };

                if !error_message.is_empty() {
                    // Try to create message (if not exists) and then edit it (if existed already)
                    warn!("Error while handling command: {:?}", error_message);
                    if let Err(e) = command
                        .create_interaction_response(&ctx.http, |response| {
                            response.kind(InteractionResponseType::ChannelMessageWithSource)
                        })
                        .await
                    {
                        debug!("Error while creating response: {:?}", e)
                    };

                    if let Err(e) = command
                        .edit_original_interaction_response(&ctx.http, |response| {
                            response.content(error_message)
                        })
                        .await
                    {
                        debug!("Error while editing response: {:?}", e)
                    };
                }
            } else {
                debug!("Invalid command received");
            }
        }
    }
}
