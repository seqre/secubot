use log::{debug, info};
use serenity::{
    async_trait,
    model::{
        application::{command::Command, interaction::Interaction},
        gateway::Ready,
        id::{ChannelId, GuildId, MessageId},
    },
    prelude::*,
};

use crate::{commands::Commands, secubot::Secubot, settings::Settings, tasks::Tasks};

pub struct Handler {
    secubot: Secubot,
    commands: Commands,
    tasks: Tasks,
    settings: Settings,
}

impl Handler {
    pub fn new(secubot: Secubot, settings: Settings) -> Self {
        let commands = Commands::new(&secubot);
        let tasks = Tasks::new();
        Self {
            secubot,
            commands,
            tasks,
            settings,
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        self.commands.handle(ctx, interaction).await;
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        for guild in &self.settings.commands.guilds {
            let guild_commands =
                GuildId::set_application_commands(&GuildId(guild.id), &ctx.http, |commands| {
                    self.commands.register_commands(commands, &guild.commands);
                    commands
                })
                .await;

            info!(
                "Guild ({}) commands: {:?}",
                guild.id,
                guild_commands
                    .unwrap()
                    .iter()
                    .map(|c| String::from(&c.name))
                    .collect::<Vec<String>>()
            );
        }

        let global_commands = Command::set_global_application_commands(&ctx.http, |commands| {
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

        self.tasks.start_tasks(&self.secubot, ctx.http.clone());
        info!("Started tasks");
    }

    #[cfg(feature = "msg_content")]
    async fn message(&self, ctx: Context, new_message: Message) {
        if new_message.is_own(&ctx.cache) {
            return;
        }

        use crate::events::handle_message;
        handle_message(self, ctx, new_message).await;
    }

    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        _deleted_message_id: MessageId,
        _guild_id: Option<GuildId>,
    ) {
        if let Err(e) = channel_id.say(&ctx, "<deleted>").await {
            debug!("Error while sending <deleted>: {:?}", e);
        };
    }

    async fn message_delete_bulk(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        deleted_message_ids: Vec<MessageId>,
        _guild_id: Option<GuildId>,
    ) {
        if let Err(e) = channel_id
            .say(&ctx, format!("<{}x deleted>", deleted_message_ids.len()))
            .await
        {
            debug!(
                "Error while sending <{}x deleted>: {:?}",
                deleted_message_ids.len(),
                e
            );
        };
    }
}
