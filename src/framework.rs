use poise::{
    serenity_prelude::{
        self as serenity, model::application::interaction::Interaction::ApplicationCommand,
        ApplicationCommandInteraction, CommandDataOption,
    },
    Event, Framework, FrameworkContext,
};
use tracing::{debug, info};

use crate::{ctx_data::CtxData, settings::Settings, tasks, Error, Result};

pub async fn on_error(error: poise::FrameworkError<'_, CtxData, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to
    // customize and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {error:?}"),
        poise::FrameworkError::Command { error, ctx } => {
            debug!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                debug!("Error while handling error: {}", e);
            }
        }
    }
}

pub async fn event_handler<'a>(
    ctx: &'a serenity::Context,
    event: &'a Event<'_>,
    _framework_context: FrameworkContext<'a, CtxData, Error>,
    _ctx_data: &'a CtxData,
) -> Result<()> {
    // debug!("Got an event in event handler: {:?}", event.name());

    // TODO: use message for URLs?
    match event {
        Event::Ready { data_about_bot } => info!("{} is connected!", data_about_bot.user.name),

        Event::InteractionCreate {
            interaction:
                ApplicationCommand(ApplicationCommandInteraction {
                    data,
                    channel_id,
                    user,
                    ..
                }),
        } => {
            let args = if data.options.is_empty() {
                String::new()
            } else {
                let mut buf = String::new();
                gather_options(&mut buf, &data.options);
                buf
            };

            debug!(
                "INTR: cmd[{}] args [{}] channelid[{}] userid[{}] username[{}]",
                data.name,
                args.trim_start(),
                channel_id.0,
                user.id,
                user.name
            );
        }

        #[allow(unused_variables)]
        Event::MessageDelete {
            channel_id,
            deleted_message_id,
            guild_id,
        } => {
            if let Err(e) = channel_id.say(&ctx, "<deleted>").await {
                debug!("Error while sending <deleted>: {:#?}", e);
            };
        }

        #[allow(unused_variables)]
        Event::MessageDeleteBulk {
            channel_id,
            multiple_deleted_messages_ids,
            guild_id,
        } => {
            let text = format!("<{}x deleted>", multiple_deleted_messages_ids.len());
            if let Err(e) = channel_id.say(&ctx, &text).await {
                debug!("Error while sending {}: {:?}", text, e);
            };
        }

        _ => (),
    };
    Ok(())
}

pub async fn setup<'a>(
    ctx: &'a serenity::Context,
    _ready: &'a serenity::Ready,
    framework: &Framework<CtxData, Error>,
    _settings: Settings,
    ctx_data: CtxData,
) -> Result<CtxData> {
    // let empty: &[poise::structs::Command<CtxData, Error>] = &[];
    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
    // TODO: add this back at some point
    // for guild in settings.commands.guilds {
    //     let commands = COMMANDS
    //         .iter()
    //         .filter(|cmd| guild.commands.contains(&cmd.name))
    //         .collect();
    //     poise::builtins::register_in_guild(ctx, commands,
    // serenity::GuildId(guild.id)).await?; }

    tasks::start_tasks(&ctx_data, ctx.http.clone());
    Ok(ctx_data)
}

fn gather_options(buf: &mut String, opts: &Vec<CommandDataOption>) {
    for opt in opts {
        buf.push(' ');
        buf.push_str(&opt.name);

        if let Some(val) = &opt.value {
            buf.push(' ');
            buf.push_str(&val.to_string());
        }

        gather_options(buf, &opt.options);
    }
}
