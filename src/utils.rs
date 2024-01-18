use poise::serenity_prelude::{GuildId, Member, User, UserId};

use crate::{Context, Result};

pub async fn get_nick_from_id(
    ctx: Context<'_>,
    guild_id: &GuildId,
    user_id: UserId,
) -> Result<String> {
    let user = user_id.to_user(ctx).await?;
    let nick = get_nick_from_user(ctx, guild_id, user).await;
    Ok(nick)
}

pub async fn get_nick_from_user(ctx: Context<'_>, guild_id: &GuildId, user: User) -> String {
    let guild_nick = user.nick_in(ctx, guild_id).await;
    guild_nick.unwrap_or(user.name)
}

pub fn get_nick_from_member(member: &Member) -> String {
    if let Some(nick) = &member.nick {
        return nick.to_string();
    }

    member.user.name.clone()
}
