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
use poise::serenity_prelude::{ChannelId, CreateEmbed, MessageBuilder};

use crate::{
    models::{NewTodo, Todo},
    Conn, Context, Result,
};

type TodoEntry = (u64, String);

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

#[poise::command(
    slash_command,
    subcommands("list", "add", "complete", "uncomplete", "delete")
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
) -> Result<()> {
    use crate::schema::todos::dsl::{channel_id, completion_date, todos};

    // FIXME: looks bad, there needs to be smarter way
    let results = if completed {
        todos
            .filter(channel_id.eq(i64::from(ctx.channel_id())))
            .load::<Todo>(&mut ctx.data().db.get().unwrap())
    } else {
        todos
            .filter(channel_id.eq(i64::from(ctx.channel_id())))
            .filter(completion_date.is_null())
            .load::<Todo>(&mut ctx.data().db.get().unwrap())
    };

    let data = match results {
        Ok(todo_list) => {
            let output: Vec<TodoEntry> = todo_list
                .into_iter()
                .map(|t| (t.id as u64, t.todo))
                .collect();
            if output.is_empty() {
                EmbedData::Text("There are no incompleted TODOs in this channel.".to_string())
            } else {
                EmbedData::Fields(output)
            }
        }
        Err(NotFound) => EmbedData::Text("Not found.".to_string()),
        Err(_) => EmbedData::Text("Listing TODOs failed.".to_string()),
    };

    respond(ctx, data).await;

    Ok(())
}

/// Add TODO entry
#[poise::command(slash_command)]
pub async fn add(ctx: Context<'_>, #[description = "TODO content"] content: String) -> Result<()> {
    use crate::schema::todos::dsl::todos;

    let data = if content.len() > 1024 {
        EmbedData::Text("Content can't have more than 1024 characters.".to_string())
    } else {
        let time = NaiveDateTime::from_timestamp_opt(Utc::now().timestamp(), 0).unwrap();
        let new_id = ctx.data().todo_data.get_id(ctx.channel_id());
        let text = content.replace('@', "@\u{200B}").replace('`', "'");
        let new_todo = NewTodo {
            channel_id: &(i64::from(ctx.channel_id())),
            id: &new_id,
            todo: &text,
            creation_date: &time.to_string(),
        };

        let result = diesel::insert_into(todos)
            .values(&new_todo)
            .execute(&mut ctx.data().db.get().unwrap());

        match result {
            Ok(_) => EmbedData::Text(
                MessageBuilder::new()
                    .push("TODO (")
                    .push_mono_safe(&text)
                    .push(") added.")
                    .build(),
            ),
            Err(NotFound) => EmbedData::Text("Not found.".to_string()),
            Err(_) => EmbedData::Text("Adding TODO failed.".to_string()),
        }
    };

    respond(ctx, data).await;

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
                .push("TODO (")
                .push_mono_safe(&deleted)
                .push(") deleted.")
                .build(),
        ),
        Err(NotFound) => EmbedData::Text("Not found.".to_string()),
        Err(_) => EmbedData::Text("Deleting TODO failed.".to_string()),
    };

    respond(ctx, data).await;

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
                .push("TODO (")
                .push_mono_safe(&completed)
                .push(") completed.")
                .build(),
        ),
        Err(NotFound) => EmbedData::Text("Not found.".to_string()),
        Err(_) => EmbedData::Text("Completing TODO failed.".to_string()),
    };

    respond(ctx, data).await;

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
                .push("TODO (")
                .push_mono_safe(&uncompleted)
                .push(") uncompleted.")
                .build(),
        ),
        Err(NotFound) => EmbedData::Text("Not found.".to_string()),
        Err(_) => EmbedData::Text("Uncompleting TODO failed.".to_string()),
    };

    respond(ctx, data).await;

    Ok(())
}

enum EmbedData {
    Text(String),
    Fields(Vec<TodoEntry>),
}

async fn respond(ctx: Context<'_>, data: EmbedData) {
    _ = ctx
        .send(|reply| reply.embed(|embed| create_embed(embed, data)))
        .await;
}

fn create_embed(builder: &mut CreateEmbed, data: EmbedData) -> &mut CreateEmbed {
    match data {
        EmbedData::Text(text) => builder.description(text),
        EmbedData::Fields(fields) => {
            let new_fields: Vec<(u64, String, bool)> = fields
                .into_iter()
                .map(|(x, y)| {
                    let b = y.len() <= 25;
                    (x, y, b)
                })
                .collect();
            builder.title("TODOs").fields(new_fields)
        }
    }
}
