use serenity::{model::channel::Message, prelude::*};
mod clean_urls;

use self::clean_urls::clean_urls;

pub async fn handle_message(_handler: &dyn EventHandler, ctx: Context, new_msg: Message) {
    let clean = clean_urls(&new_msg.content);
    if !clean.is_empty() {
        let _ = &ctx
            .http
            .get_message(new_msg.channel_id.0, new_msg.id.0)
            .await
            .unwrap()
            .suppress_embeds(&ctx.http)
            .await;
        let _ = new_msg.reply(&ctx.http, clean).await;
    }
}
