use octocrab::Octocrab;
use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    client::Context,
    model::application::interaction::{
        application_command::ApplicationCommandInteraction, InteractionResponseType,
    },
};

use crate::commands::{Command, CommandResult};

const CHANGELOG_COMMAND: &str = "changelog";
const CHANGELOG_COMMAND_DESC: &str = "Get the latest changelog";

#[derive(Debug)]
pub struct ChangelogCommand {
    octocrab: Octocrab,
}

impl ChangelogCommand {
    pub fn new() -> Self {
        Self {
            octocrab: Octocrab::builder().build().unwrap(),
        }
    }

    pub async fn get_changelog(&self) -> Result<String, octocrab::Error> {
        let release = &self
            .octocrab
            .repos("seqre", "secubot")
            .releases()
            .get_latest()
            .await?;
        let changelog = release
            .body
            .as_ref()
            .unwrap()
            .split("\r\n")
            .into_iter()
            .map(|x| {
                if x.starts_with('#') {
                    format!("**{}**\n", x)
                } else {
                    format!("{}\n", x)
                }
            })
            .collect();

        Ok(changelog)
    }
}

#[async_trait]
impl Command for ChangelogCommand {
    fn get_name(&self) -> &'static str {
        CHANGELOG_COMMAND
    }

    fn add_application_command(&self, command: &mut CreateApplicationCommand) {
        command
            .name(CHANGELOG_COMMAND)
            .description(CHANGELOG_COMMAND_DESC);
    }

    async fn handle(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> CommandResult {
        let response_text = &self.get_changelog().await?;

        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.content(response_text);
                        message
                    })
            })
            .await?;

        Ok(())
    }
}
