use log::debug;
use poise::{serenity_prelude as serenity, Event, Framework, FrameworkContext};

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
                debug!("Error while handling error: {}", e)
            }
        }
    }
}

pub async fn event_handler<'a>(
    _ctx: &'a serenity::Context,
    event: &'a Event<'_>,
    _framework_context: FrameworkContext<'a, CtxData, Error>,
    _ctx_data: &'a CtxData,
) -> Result<()> {
    debug!("Got an event in event handler: {:?}", event.name());
    Ok(())
}

pub async fn setup<'a>(
    ctx: &'a serenity::Context,
    _ready: &'a serenity::Ready,
    framework: &Framework<CtxData, Error>,
    settings: Settings,
    ctx_data: CtxData,
) -> Result<CtxData> {
    let empty: &[poise::structs::Command<CtxData, Error>] = &[];
    poise::builtins::register_globally(ctx, empty).await?;
    poise::builtins::register_in_guild(
        ctx,
        &framework.options().commands,
        serenity::GuildId(settings.commands.guilds[0].id),
    )
    .await?;
    tasks::start_tasks(&ctx_data, ctx.http.clone());
    Ok(ctx_data)
}
