use log::debug;

use crate::{ctx_data::CtxData, Error};

pub async fn on_error(error: poise::FrameworkError<'_, CtxData, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to
    // customize and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
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
