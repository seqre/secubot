use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI32, Ordering},
        Mutex,
    },
};

use chrono::{NaiveDateTime, Utc};
use diesel::{
    prelude::*,
    result::{Error::NotFound, QueryResult},
};
use itertools::Itertools;
use poise::serenity_prelude::{
    ChannelId, CreateEmbed, GuildChannel, Member, MessageBuilder, UserId,
};
use tokio_stream::{self as stream, StreamExt};

use crate::{
    models::{NewTodo, Todo},
    Conn, Context, Result,
};

struct TodoEntry {
    id: i32,
    assignee: Option<String>,
    text: String,
}

impl TodoEntry {
    pub async fn new(id: i32, assignee: Option<i64>, text: String, ctx: Context<'_>) -> Self {
        let assignee = match assignee {
            Some(id) => {
                let userid = UserId(id as u64);
                let guildid = ctx.guild_id().unwrap();
                let member = guildid.member(ctx, userid).await.unwrap();
                Some(get_member_nickname(&member))
            }
            None => None,
        };
        Self { id, assignee, text }
    }
}

#[derive(Debug)]
pub struct TodoData {
    iterators: Mutex<HashMap<ChannelId, AtomicI32>>,
}

impl TodoData {
    pub fn new(db: &Conn) -> Self {
        use crate::schema::todos::dsl::todos;

        let todo_list = todos.load::<Todo>(&mut db.get().unwrap()).unwrap();
        let iterators = todo_list
            .into_iter()
            .group_by(|td| td.channel_id)
            .into_iter()
            .map(|(chnl, tds)| {
                let biggest_id = tds.map(|t| t.id).max().unwrap_or(0);
                (ChannelId(chnl as u64), AtomicI32::new(biggest_id + 1))
            })
            .collect::<HashMap<_, _>>();

        Self {
            iterators: Mutex::new(iterators),
        }
    }

    fn get_id(&self, channelid: ChannelId) -> i32 {
        let itr = &mut self.iterators.lock().unwrap();
        let aint = itr.entry(channelid).or_insert_with(|| AtomicI32::new(0));
        aint.fetch_add(1, Ordering::SeqCst)
    }
}

#[allow(clippy::unused_async)]
#[poise::command(
    slash_command,
    subcommands("list", "add", "complete", "uncomplete", "delete", "assign", "rmove")
)]
pub async fn todo(_ctx: Context<'_>) -> Result<()> {
    Ok(())
}

// TODO: division of responsibilites, extract database manipulations to other
// functions

/// List TODO entries
#[poise::command(slash_command)]
pub async fn list(
    ctx: Context<'_>,
    #[description = "Show completed TODOs"]
    #[flag]
    completed: bool,
    #[description = "Show only TODOs assigned to"] todo_assignee: Option<Member>,
) -> Result<()> {
    use crate::schema::todos::dsl::{assignee, channel_id, completion_date, todos};

    let mut query = todos
        .into_boxed()
        .filter(channel_id.eq(i64::from(ctx.channel_id())));

    if !completed {
        query = query.filter(completion_date.is_null());
    };

    if let Some(member) = todo_assignee {
        query = query.filter(assignee.eq(member.user.id.0 as i64));
    };

    let results = query.load::<Todo>(&mut ctx.data().db.get().unwrap());

    let data = match results {
        Ok(todo_list) => {
            let mut output: Vec<TodoEntry> = vec![];
            let mut todos_stream = stream::iter(todo_list);

            while let Some(t) = todos_stream.next().await {
                let entry = TodoEntry::new(t.id, t.assignee, t.todo, ctx).await;
                output.push(entry);
            }

            if output.is_empty() {
                EmbedData::Text("There are no incompleted TODOs in this channel.".to_string())
            } else {
                EmbedData::Fields(output)
            }
        }
        Err(NotFound) => EmbedData::Text("Not found.".to_string()),
        Err(_) => EmbedData::Text("Listing TODOs failed.".to_string()),
    };

    respond(ctx, data, false).await;

    Ok(())
}

/// Add TODO entry
#[poise::command(slash_command)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "TODO content"] content: String,
    #[description = "TODO assignee"] assignee: Option<Member>,
) -> Result<()> {
    use crate::schema::todos::dsl::todos;

    let data = if content.len() > 1024 {
        EmbedData::Text("Content can't have more than 1024 characters.".to_string())
    } else {
        let time = NaiveDateTime::from_timestamp_opt(Utc::now().timestamp(), 0).unwrap();
        let new_id = ctx.data().todo_data.get_id(ctx.channel_id());
        let text = content.replace('@', "@\u{200B}").replace('`', "'");
        let nickname = match &assignee {
            Some(m) => get_member_nickname(m),
            None => "no one".to_string(),
        };
        let assignee = assignee.map(|m| m.user.id.0 as i64);

        let new_todo = NewTodo {
            channel_id: &(i64::from(ctx.channel_id())),
            id: &new_id,
            todo: &text,
            creation_date: &time.to_string(),
            assignee,
        };

        let result = diesel::insert_into(todos)
            .values(&new_todo)
            .execute(&mut ctx.data().db.get().unwrap());

        match result {
            Ok(_) => EmbedData::Text(
                MessageBuilder::new()
                    .push(format!("TODO [{}] (", &new_id))
                    .push_mono_safe(&text)
                    .push(format!(") added and assigned to {nickname}."))
                    .build(),
            ),
            Err(NotFound) => EmbedData::Text("Not found.".to_string()),
            Err(_) => EmbedData::Text("Adding TODO failed.".to_string()),
        }
    };

    respond(ctx, data, false).await;

    Ok(())
}

/// Delete TODO entry
#[poise::command(slash_command)]
pub async fn delete(ctx: Context<'_>, #[description = "TODO id"] todo_id: i64) -> Result<()> {
    use crate::schema::todos::dsl::{channel_id, id, todo, todos};

    let deleted: QueryResult<String> = diesel::delete(todos)
        .filter(channel_id.eq(i64::from(ctx.channel_id())))
        .filter(id.eq(todo_id as i32))
        .returning(todo)
        .get_result(&mut ctx.data().db.get().unwrap());

    let data = match deleted {
        Ok(deleted) => EmbedData::Text(
            MessageBuilder::new()
                .push(format!("TODO [{}] (", &todo_id))
                .push_mono_safe(&deleted)
                .push(") deleted.")
                .build(),
        ),
        Err(NotFound) => EmbedData::Text("Not found.".to_string()),
        Err(_) => EmbedData::Text("Deleting TODO failed.".to_string()),
    };

    respond(ctx, data, true).await;

    Ok(())
}

/// Complete TODO entry
#[poise::command(slash_command)]
pub async fn complete(ctx: Context<'_>, #[description = "TODO id"] todo_id: i64) -> Result<()> {
    use crate::schema::todos::dsl::{channel_id, completion_date, id, todo, todos};

    let time = NaiveDateTime::from_timestamp_opt(Utc::now().timestamp(), 0).unwrap();

    let completed: QueryResult<String> = diesel::update(todos)
        .filter(channel_id.eq(i64::from(ctx.channel_id())))
        .filter(id.eq(todo_id as i32))
        .set(completion_date.eq(&time.to_string()))
        .returning(todo)
        .get_result(&mut ctx.data().db.get().unwrap());

    let data = match completed {
        Ok(completed) => EmbedData::Text(
            MessageBuilder::new()
                .push(format!("TODO [{}] (", &todo_id))
                .push_mono_safe(&completed)
                .push(") completed.")
                .build(),
        ),
        Err(NotFound) => EmbedData::Text("Not found.".to_string()),
        Err(_) => EmbedData::Text("Completing TODO failed.".to_string()),
    };

    respond(ctx, data, false).await;

    Ok(())
}

/// Uncomplete TODO entry
#[poise::command(slash_command)]
pub async fn uncomplete(ctx: Context<'_>, #[description = "TODO id"] todo_id: i64) -> Result<()> {
    use crate::schema::todos::dsl::{channel_id, completion_date, id, todo, todos};

    let uncompleted: QueryResult<String> = diesel::update(todos)
        .filter(channel_id.eq(i64::from(ctx.channel_id())))
        .filter(id.eq(todo_id as i32))
        .set(completion_date.eq::<Option<String>>(None))
        .returning(todo)
        .get_result(&mut ctx.data().db.get().unwrap());

    let data = match uncompleted {
        Ok(uncompleted) => EmbedData::Text(
            MessageBuilder::new()
                .push(format!("TODO [{}] (", &todo_id))
                .push_mono_safe(&uncompleted)
                .push(") uncompleted.")
                .build(),
        ),
        Err(NotFound) => EmbedData::Text("Not found.".to_string()),
        Err(_) => EmbedData::Text("Uncompleting TODO failed.".to_string()),
    };

    respond(ctx, data, true).await;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn assign(
    ctx: Context<'_>,
    #[description = "TODO id"] todo_id: i64,
    #[description = "TODO new assignee"] new_assignee: Option<Member>,
) -> Result<()> {
    use crate::schema::todos::dsl::{assignee, channel_id, id, todo, todos};

    let nickname = match &new_assignee {
        Some(m) => get_member_nickname(m),
        None => "no one".to_string(),
    };
    let new_assignee = new_assignee.map(|m| m.user.id.0 as i64);

    let reassigned: QueryResult<String> = diesel::update(todos)
        .filter(channel_id.eq(i64::from(ctx.channel_id())))
        .filter(id.eq(todo_id as i32))
        .set(assignee.eq(new_assignee))
        .returning(todo)
        .get_result(&mut ctx.data().db.get().unwrap());

    let data = match reassigned {
        Ok(reassigned) => EmbedData::Text(
            MessageBuilder::new()
                .push(format!("TODO [{}] (", &todo_id))
                .push_mono_safe(&reassigned)
                .push(format!(") reassigned to {nickname}."))
                .build(),
        ),
        Err(NotFound) => EmbedData::Text("Not found.".to_string()),
        Err(_) => EmbedData::Text("Assigning TODO failed.".to_string()),
    };

    respond(ctx, data, true).await;

    Ok(())
}

#[poise::command(slash_command, rename = "move")]
pub async fn rmove(
    ctx: Context<'_>,
    #[description = "TODO id"] todo_id: i64,
    #[description = "TODO new channel"] new_channel: GuildChannel,
) -> Result<()> {
    use crate::schema::todos::dsl::*;

    let new_channel_id = new_channel.id.0 as i64;
    let new_id = ctx.data().todo_data.get_id(new_channel.id);

    let moved: QueryResult<String> = diesel::update(todos)
        .filter(channel_id.eq(i64::from(ctx.channel_id())))
        .filter(id.eq(todo_id as i32))
        .set((channel_id.eq(new_channel_id), id.eq(new_id)))
        .returning(todo)
        .get_result(&mut ctx.data().db.get().unwrap());

    let data = match moved {
        Ok(moved) => EmbedData::Text(
            MessageBuilder::new()
                .push(format!("TODO [{}] (", &todo_id))
                .push_mono_safe(&moved)
                .push(format!(") moved to {}.", new_channel.name()))
                .build(),
        ),
        Err(NotFound) => EmbedData::Text("Not found.".to_string()),
        Err(_) => EmbedData::Text("Moving TODO failed.".to_string()),
    };

    respond(ctx, data, true).await;

    Ok(())
}

fn get_member_nickname(member: &Member) -> String {
    if let Some(nick) = &member.nick {
        return nick.to_string();
    }

    member.user.name.to_string()
}

enum EmbedData {
    Text(String),
    Fields(Vec<TodoEntry>),
}

async fn respond(ctx: Context<'_>, data: EmbedData, ephemeral: bool) {
    _ = ctx
        .send(|reply| {
            reply
                .embed(|embed| create_embed(embed, data))
                .ephemeral(ephemeral)
        })
        .await;
}

fn create_embed(builder: &mut CreateEmbed, data: EmbedData) -> &mut CreateEmbed {
    match data {
        EmbedData::Text(text) => builder.description(text),
        EmbedData::Fields(fields) => {
            let new_fields: Vec<(String, String, bool)> = fields
                .into_iter()
                .map(|entry| {
                    let inline = entry.text.len() <= 25;
                    let mut name = format!("[{}]", entry.id);
                    if let Some(nick) = entry.assignee {
                        name = format!("{name} - {nick}");
                    };
                    (name, entry.text, inline)
                })
                .collect();
            builder.title("TODOs").fields(new_fields)
        }
    }
}
