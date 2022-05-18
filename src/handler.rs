use log::info;
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

pub struct Handler {
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
        info!("{} is connected!", ready.user.name);

        info!("Guild slash commands:");
        for guild in &self.settings.commands.guilds {
            let guild_commands =
                GuildId::set_application_commands(&GuildId(guild.id), &ctx.http, |commands| {
                    self.commands.register_commands(commands, &guild.commands);
                    commands
                })
                .await;

            info!(
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

        info!(
            "Global slash commands: {:?}",
            global_commands
                .unwrap()
                .iter()
                .map(|c| String::from(&c.name))
                .collect::<Vec<String>>()
        );
    }
}
