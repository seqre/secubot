use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use diesel::{prelude::*, ExpressionMethods};
use itertools::Itertools;
use poise::serenity_prelude::{GuildId, User};
use time::OffsetDateTime;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::{
    ctx_data::CtxData,
    models::hall_of_fame::{NewEntry, NewTable, Table},
    Conn, Context, Error, Result,
};

#[derive(Debug)]
pub struct HofData {
    hofs: RwLock<HashMap<GuildId, HashSet<String>>>,
}

impl HofData {
    pub async fn add_table(&self, guild_id: GuildId, table: String) {
        self.hofs
            .write()
            .await
            .entry(guild_id)
            .and_modify(|hofs| {
                hofs.insert(table);
            })
            .or_default();
    }

    pub async fn get_hof_tables(&self, guild_id: &GuildId) -> HashSet<String> {
        // TODO: allow dirty read
        self.hofs
            .read()
            .await
            .get(&guild_id)
            .map(|h| h.clone())
            .unwrap_or_default()
    }
}

impl HofData {
    pub fn new(db: &Conn) -> Self {
        use crate::schema::hall_of_fame_tables::dsl::*;

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
}

#[allow(clippy::unused_async)]
#[poise::command(slash_command, subcommands("show", "create", "add"))]
pub async fn hof(_ctx: Context<'_>) -> Result<()> {
    Ok(())
}

async fn autocomplete<'a>(ctx: Context<'_>, partial: &'a str) -> HashSet<String> {
    let guild = ctx.guild_id().unwrap();

    ctx.data()
        .hof_data
        .get_hof_tables(&guild)
        .await
        .into_iter()
        .filter(|h| h.starts_with(partial))
        .collect()
}

/// List Hall of Fame tables
#[poise::command(slash_command)]
pub async fn show(ctx: Context<'_>, #[autocomplete = "autocomplete"] hof: String) -> Result<()> {
    use crate::{
        models::hall_of_fame::Entry,
        schema::{hall_of_fame_entries::dsl::*, hall_of_fame_tables::dsl::*},
    };

    let guild = ctx.guild_id().unwrap();

    let hof = hall_of_fame_tables
        .filter(guild_id.eq::<i64>(guild.into()))
        .filter(title.eq(&hof))
        .first::<Table>(&mut ctx.data().db.get().unwrap())?;

    debug!("{:#?}", hof);

    ctx.reply(format!("{:?}", hof)).await?;
    // TODO: embed

    let entries = hall_of_fame_entries
        .filter(hof_id.eq(hof.id))
        .load::<Entry>(&mut ctx.data().db.get().unwrap())?;

    ctx.reply(format!("entries: {:?}", entries)).await?;

    Ok(())
}

#[derive(Debug, poise::Modal)]
#[name = "Create Hall of Fame table"]
struct HofCreationModal {
    #[min_length = 5]
    #[max_length = 100]
    title: String,
    #[paragraph]
    #[max_length = 500]
    description: Option<String>,
}

#[poise::command(slash_command)]
pub async fn create(ctx: poise::ApplicationContext<'_, Arc<CtxData>, Error>) -> Result<()> {
    use poise::Modal as _;

    use crate::schema::hall_of_fame_tables::dsl::*;

    let data = HofCreationModal::execute(ctx).await?;

    if let Some(data) = data {
        let guild = ctx.guild_id().unwrap();
        // let time = OffsetDateTime::now_utc()
        //    .format(&TIME_FORMAT.get().unwrap())
        //    .unwrap();

        let new_hof = NewTable {
            guild_id: &(guild.0 as i64),
            title: &data.title,
            description: data.description,
            creation_date: &"", // TODO: fix
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

        ctx.reply(response).await?;
    }

    return Ok(());
}

#[poise::command(slash_command)]
pub async fn add(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete"] hof: String,
    user: User,
    reason: String,
) -> Result<()> {
    use crate::schema::{hall_of_fame_entries::dsl::*, hall_of_fame_tables::dsl::*};
    let guild = ctx.guild_id().unwrap();

    debug!("u:{user} r:{reason}");

    if reason.len() > 256 {
        ctx.reply(
            "Reason is too long, it can be 256 characters long at
most.",
        )
        .await?;
        return Ok(());
    }

    let hof = hall_of_fame_tables
        .filter(guild_id.eq::<i64>(guild.into()))
        .filter(title.eq(&hof))
        .first::<Table>(&mut ctx.data().db.get().unwrap())?;

    let new_entry = NewEntry {
        hof_id: &hof.id,
        user_id: &(user.id.0 as i64),
        description: Some(&reason),
        creation_date: &"",
    };

    let result = diesel::insert_into(hall_of_fame_entries)
        .values(&new_entry)
        .execute(&mut ctx.data().db.get().unwrap());

    debug!("{:?}", result);

    Ok(())
}
