use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use diesel::{prelude::*, ExpressionMethods};
use itertools::Itertools;
use poise::serenity_prelude::{GuildId, MessageBuilder, User, UserId};
use time::OffsetDateTime;
use tokio::sync::RwLock;

use crate::{
    commands::{DISCORD_EMBED_FIELDS_LIMIT, TIME_FORMAT},
    ctx_data::CtxData,
    models::hall_of_fame::{Entry, NewEntry, NewTable, Table},
    Conn, Context, Error, Result,
};

#[derive(Debug)]
pub struct HofData {
    hofs: RwLock<HashMap<GuildId, HashSet<String>>>,
}

impl HofData {
    pub fn new(db: &Conn) -> Self {
        use crate::schema::hall_of_fame_tables::dsl::hall_of_fame_tables;

        let hofs = hall_of_fame_tables
            .load::<Table>(&mut db.get().unwrap())
            .unwrap();

        let hofs = hofs
            .into_iter()
            .group_by(|h| h.guild_id)
            .into_iter()
            .map(|(grp, hfs)| (GuildId(grp as u64), hfs.map(|h| h.title).collect()))
            .collect();

        Self {
            hofs: RwLock::new(hofs),
        }
    }

    pub async fn add_table(&self, guild_id: GuildId, table: String) {
        let mut hofs = self.hofs.write().await;
        hofs.entry(guild_id).or_default().insert(table);
    }

    pub async fn get_hof_tables(&self, guild_id: &GuildId) -> HashSet<String> {
        // TODO: allow dirty read
        self.hofs
            .read()
            .await
            .get(guild_id)
            .cloned()
            .unwrap_or_default()
    }
}

#[allow(clippy::unused_async)]
#[poise::command(slash_command, subcommands("show", "create", "add"))]
pub async fn hof(_ctx: Context<'_>) -> Result<()> {
    Ok(())
}

async fn autocomplete<'a>(ctx: Context<'_>, partial: &'a str) -> HashSet<String> {
    let guild = match ctx.guild_id() {
        Some(guild) => guild,
        None => return HashSet::new(),
    };

    ctx.data()
        .hof_data
        .get_hof_tables(&guild)
        .await
        .into_iter()
        .filter(|h| h.starts_with(partial))
        .sorted()
        .collect()
}

/// List Hall of Fame tables
#[poise::command(slash_command)]
pub async fn show(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete"] hof: String,
    user: Option<User>,
) -> Result<()> {
    let guild = ctx.guild_id().unwrap();

    match user {
        None => show_hof(ctx, guild, hof).await?,
        Some(user_id) => show_user(ctx, guild, hof, user_id).await?,
    };

    Ok(())
}

async fn show_hof(ctx: Context<'_>, guild: GuildId, hof: String) -> Result<()> {
    use crate::{
        schema::{
            hall_of_fame_entries::dsl::{hall_of_fame_entries, hof_id},
            hall_of_fame_tables::dsl::{guild_id, hall_of_fame_tables, title},
        },
        utils,
    };

    let hof = hall_of_fame_tables
        .filter(guild_id.eq::<i64>(guild.into()))
        .filter(title.eq(&hof))
        .first::<Table>(&mut ctx.data().db.get().unwrap())?;

    let entries = hall_of_fame_entries
        .filter(hof_id.eq(hof.id))
        .load::<Entry>(&mut ctx.data().db.get().unwrap())?;

    let entries: Vec<_> = entries
        .into_iter()
        .counts_by(|e| e.user_id)
        .into_iter()
        .sorted_by_cached_key(|(_, v)| -(*v as i32))
        .take(DISCORD_EMBED_FIELDS_LIMIT as usize)
        .collect();

    let mut entries2 = vec![];
    for (k, v) in entries {
        entries2.push((
            utils::get_nick_from_id(ctx, &guild, UserId(k as u64)).await?,
            v,
            true,
        ));
    }

    let _response = ctx
        .send(|reply| {
            reply.embed(|embed| {
                let desc = hof.description.unwrap_or_default();
                let desc = if entries2.is_empty() {
                    let mix = if desc.is_empty() { "" } else { "\n\n" };
                    format!("{desc}{mix}There are no entries.")
                } else {
                    desc
                };
                embed.title(&hof.title).description(desc).fields(entries2)
            })
        })
        .await?;

    Ok(())
}
async fn show_user(ctx: Context<'_>, guild: GuildId, hof: String, user: User) -> Result<()> {
    use crate::schema::{
        hall_of_fame_entries::dsl::{hall_of_fame_entries, hof_id, user_id},
        hall_of_fame_tables::dsl::{guild_id, hall_of_fame_tables, title},
    };

    let hof = hall_of_fame_tables
        .filter(guild_id.eq::<i64>(guild.into()))
        .filter(title.eq(&hof))
        .first::<Table>(&mut ctx.data().db.get().unwrap())?;

    let entries = hall_of_fame_entries
        .filter(hof_id.eq(hof.id))
        .filter(user_id.eq(user.id.0 as i64))
        .load::<Entry>(&mut ctx.data().db.get().unwrap())?;

    let entries: Vec<_> = entries
        .into_iter()
        .map(|e| {
            format!(
                "*{}*: {}",
                e.creation_date,
                e.description.unwrap_or(String::from("Missing reason"))
            )
        })
        .collect();

    let mut msg = MessageBuilder::new();
    msg.push(format!("### {} entries for ", hof.title))
        .mention(&user)
        .push_line("");

    for entry in entries.iter().rev() {
        msg.push_line(format!("- {entry}"));
    }

    ctx.reply(msg.build()).await?;

    Ok(())
}

#[derive(Debug, poise::Modal)]
#[name = "Create Hall of Fame table"]
struct HofCreationModal {
    #[min_length = 4]
    #[max_length = 64]
    title: String,
    #[paragraph]
    #[max_length = 128]
    description: Option<String>,
}

#[poise::command(slash_command)]
pub async fn create(ctx: poise::ApplicationContext<'_, Arc<CtxData>, Error>) -> Result<()> {
    use poise::Modal as _;

    use crate::schema::hall_of_fame_tables::dsl::hall_of_fame_tables;

    let data = HofCreationModal::execute(ctx).await?;

    if let Some(data) = data {
        let guild = ctx.guild_id().unwrap();
        let time = OffsetDateTime::now_utc().format(&TIME_FORMAT).unwrap();

        let desc = match data.description {
            Some(s) => {
                let desc = s.trim();
                if desc.is_empty() {
                    None
                } else {
                    Some(desc.to_string())
                }
            }
            None => None,
        };

        let new_hof = NewTable {
            guild_id: &(guild.0 as i64),
            title: &data.title,
            description: desc,
            creation_date: &time,
        };

        let result = diesel::insert_into(hall_of_fame_tables)
            .values(&new_hof)
            .execute(&mut ctx.data().db.get().unwrap());

        let response = match result {
            Ok(_) => {
                ctx.data.hof_data.add_table(guild, data.title).await;
                "Success"
            }
            _ => "Failure",
        };

        ctx.send(|reply| reply.content(response).reply(true).ephemeral(true))
            .await?;
    }

    Ok(())
}

#[poise::command(slash_command)]
pub async fn add(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete"] hof: String,
    user: User,
    #[max_length = 128] reason: String,
) -> Result<()> {
    use crate::schema::{
        hall_of_fame_entries::dsl::hall_of_fame_entries,
        hall_of_fame_tables::dsl::{guild_id, hall_of_fame_tables, title},
    };
    let guild = ctx.guild_id().unwrap();
    let time = OffsetDateTime::now_utc().format(&TIME_FORMAT).unwrap();

    let hof = hall_of_fame_tables
        .filter(guild_id.eq::<i64>(guild.into()))
        .filter(title.eq(&hof))
        .first::<Table>(&mut ctx.data().db.get().unwrap())?;

    let new_entry = NewEntry {
        hof_id: &hof.id,
        user_id: &(user.id.0 as i64),
        description: Some(&reason),
        creation_date: &time,
    };

    let _result = diesel::insert_into(hall_of_fame_entries)
        .values(&new_entry)
        .execute(&mut ctx.data().db.get().unwrap());

    let msg = MessageBuilder::new()
        .mention(&user)
        .push(" was added to ")
        .push_bold(&hof.title)
        .push(": ")
        .push_italic_safe(&reason)
        .build();

    ctx.reply(msg).await?;

    Ok(())
}
