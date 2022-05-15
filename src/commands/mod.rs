use async_trait::async_trait;
use serenity::{
    builder::{CreateApplicationCommand, CreateApplicationCommands},
    client::Context,
    model::interactions::{
        application_command::ApplicationCommandInteraction, Interaction, InteractionResponseType,
    },
};
use std::error::Error;

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
    commands: Vec<Box<dyn Command>>,
}

impl Commands {
    pub fn new(secubot: &Secubot) -> Self {
        Self {
            commands: Self::get_commands(secubot),
        }
    }

    fn get_commands(secubot: &Secubot) -> Vec<Box<dyn Command>> {
        vec![
            Box::new(TodoCommand::new(secubot)),
            Box::new(PingCommand::new()),
        ]
    }

    pub fn register_commands(&self, commands: &mut CreateApplicationCommands) {
        for command in &self.commands {
            commands.create_application_command(|com| {
                command.add_application_command(com);
                com.name(command.get_name())
            });
        }
    }

    pub async fn handle(&self, ctx: Context, interaction: Interaction, secubot: &Secubot) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let requested_command_name = command.data.name.as_str();
            let bot_command_option = self
                .commands
                .iter()
                .find(|command| command.get_name() == requested_command_name);

            if let Some(bot_command) = bot_command_option {
                let error_message =
                    if let Err(e) = bot_command.handle(&ctx, &command, secubot).await {
                        println!("Could not respond: {:?}", e);
                        format!("Could not generate response:\n```\n{}\n```", e)
                    } else {
                        String::from("")
                    };

                if !error_message.is_empty() {
                    // Try to create message (if not exists) and then edit it (if existed already)
                    command
                        .create_interaction_response(&ctx.http, |response| {
                            response.kind(InteractionResponseType::ChannelMessageWithSource)
                        })
                        .await;

                    command
                        .edit_original_interaction_response(&ctx.http, |response| {
                            response.content(error_message)
                        })
                        .await;
                }
            } else {
                println!("Invalid command received");
            }
        }
    }
}
