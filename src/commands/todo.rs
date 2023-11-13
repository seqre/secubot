#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI32, Ordering},
        Mutex, OnceLock,
    },
    time::Duration,
};

use diesel::{
    prelude::*,
    result::{Error::NotFound, QueryResult},
};
use itertools::Itertools;
use poise::{
    async_trait,
    serenity_prelude::{
        json, ButtonStyle, ChannelId, GuildChannel, Member, MessageBuilder, UserId,
    },
    SlashArgument,
};
use time::{format_description, format_description::FormatItem, OffsetDateTime};
use tokio_stream::{self as stream, StreamExt};
use tracing::debug;

use crate::{
    commands::DISCORD_EMBED_FIELDS_LIMIT,
    models::todo::{NewTodo, Todo},
    Conn, Context, Result,
};

static TIME_FORMAT: OnceLock<Vec<FormatItem<'static>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, Default)]
pub enum Priority {
    #[default]
    None,
    Low,
    Medium,
    High,
}

impl Priority {
    const VALUES: [&'static str; 4] = ["None", "Low", "Medium", "High"];
}

impl From<i32> for Priority {
    fn from(value: i32) -> Self {
        match value {
            0 => Priority::None,
            1 => Priority::Low,
            2 => Priority::Medium,
            3 => Priority::High,
            _ => unreachable!(),
        }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", Priority::VALUES[*self as usize])
    }
}

#[async_trait]
impl SlashArgument for Priority {
    fn choices() -> Vec<poise::CommandParameterChoice> {
        Priority::VALUES
            .iter()
            .map(|&s| poise::CommandParameterChoice {
                name: String::from(s),
                localizations: HashMap::default(),
            })
            .collect()
    }

    fn create(builder: &mut poise::serenity_prelude::CreateApplicationCommandOption) {
        builder.kind(poise::serenity_prelude::CommandOptionType::Number);
    }

    async fn extract<'life0, 'life1, 'life2>(
        _ctx: &'life0 poise::serenity_prelude::Context,
        _interaction: poise::ApplicationCommandOrAutocompleteInteraction<'life1>,
        value: &'life2 poise::serenity_prelude::json::Value,
    ) -> core::result::Result<Self, poise::SlashArgError>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
    {
        let priority = if let json::Value::Number(num) = value {
            num.as_f64().unwrap() as i32
        } else {
            0
        };

        Ok(Priority::from(priority))
    }
}

#[derive(Debug, PartialEq)]
struct TodoEntry {
    id: i32,
    assignee: Option<String>,
    text: String,
    completed: bool,
    priority: i32,
}

impl TodoEntry {
    pub async fn new(
        id: i32,
        assignee: Option<i64>,
        text: String,
        completed: bool,
        priority: i32,
        ctx: Context<'_>,
    ) -> Self {
        let assignee = match assignee {
            Some(id) => {
                let userid = UserId(id as u64);
                let guildid = ctx.guild_id().unwrap();
                let member = guildid.member(ctx, userid).await;
                member.ok().map(|m| get_member_nickname(&m))
            }
            None => None,
        };
        let completed = completed | false;
        Self {
            id,
            assignee,
            text,
            completed,
            priority,
        }
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

        let _ = TIME_FORMAT.set(
            format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap(),
        );

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

// The following docs are done to prevent line breaks.

/// Manage channel TODOs
#[doc = ""]
#[doc = "The following commands are supported (`{}` indicate mandatory argument, `[]` indicate optional argument):"]
#[doc = "- `/todo list [completed] [todo_assignee]` - lists all TODOs in the channel, `completed` flag set to True includes completed TODOs in the list, `todo_assignee` field set to someone will show only TODOs assigned to them"]
#[doc = "- `/todo add {content} [assignee]` - adds new TODO in the channel, `content` field is required and contains the TODO text, you can assign it to a specific person by using `assignee` field"]
#[doc = "- `/todo complete {id}` - completes TODO specified by `id`"]
#[doc = "- `/todo uncomplete {id}` - uncompletes TODO specified by `id`"]
#[doc = "- `/todo delete {id}` - deletes TODO specified by `id`"]
#[doc = "- `/todo assign {id} {new_assignee}` - assignees TODO specified by `id` to `new_assignee`"]
#[doc = "- `/todo move {id} {new_channel}` - moves TODO specified by `id` to `new_channel`"]
#[doc = "- `/todo edit {id} {new_content}` - replaces content of TODO specified by `id` to {new_content}"]
#[doc = "- `/todo set_priority {id} {new_priority}` - sets priority of TODO specified by `id` to `new_priority`"]
#[allow(clippy::unused_async)]
#[poise::command(
    slash_command,
    subcommands(
        "list",
        "add",
        "complete",
        "uncomplete",
        "delete",
        "assign",
        "rmove",
        "edit",
        "set_priority"
    )
)]
pub async fn todo(_ctx: Context<'_>) -> Result<()> {
    Ok(())
}

// TODO: division of responsibilites, extract database manipulations to other
// functions

struct QueryData {
    completed: bool,
    todo_assignee: Option<Member>,
    sort_by_priority: bool,
}

/// List TODO entries
#[poise::command(slash_command)]
pub async fn list(
    ctx: Context<'_>,
    #[description = "Show completed TODOs"]
    #[flag]
    completed: bool,
    #[description = "Show only TODOs assigned to"] todo_assignee: Option<Member>,
    #[description = "Sort TODOs by priority"]
    #[flag]
    sort_by_priority: bool,
) -> Result<()> {
    let query_data = QueryData {
        completed,
        todo_assignee,
        sort_by_priority,
    };
    let data = get_todos(ctx, &query_data).await;

    match data {
        EmbedData::Text(text) => respond_text(ctx, text, false).await,
        EmbedData::Fields(fields) => respond_fields(ctx, fields, query_data).await,
    };

    Ok(())
}

async fn get_todos(ctx: Context<'_>, query_data: &QueryData) -> EmbedData {
    use crate::schema::todos::dsl::{assignee, channel_id, completion_date, todos};

    let mut query = todos
        .into_boxed()
        .filter(channel_id.eq(i64::from(ctx.channel_id())));

    if !query_data.completed {
        query = query.filter(completion_date.is_null());
    };

    if let Some(member) = &query_data.todo_assignee {
        query = query.filter(assignee.eq(member.user.id.0 as i64));
    };

    let results = query.load::<Todo>(&mut ctx.data().db.get().unwrap());

    match results {
        Ok(todo_list) => {
            let mut output: Vec<TodoEntry> = vec![];
            let mut todos_stream = stream::iter(todo_list);

            while let Some(t) = todos_stream.next().await {
                let completed = t.completion_date.is_some();
                let entry =
                    TodoEntry::new(t.id, t.assignee, t.todo, completed, t.priority, ctx).await;
                output.push(entry);
            }

            if query_data.sort_by_priority {
                output.sort_by_key(|entry| -entry.priority);
            }

            if output.is_empty() {
                EmbedData::Text("There are no incompleted TODOs in this channel.".to_string())
            } else {
                EmbedData::Fields(output)
            }
        }
        Err(NotFound) => EmbedData::Text("Not found.".to_string()),
        Err(_) => EmbedData::Text("Listing TODOs failed.".to_string()),
    }
}

/// Add TODO entry
#[poise::command(slash_command)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "TODO content"] content: String,
    #[description = "TODO assignee"] assignee: Option<Member>,
    #[description = "TODO priority"] priority: Option<Priority>,
) -> Result<()> {
    use crate::schema::todos::dsl::todos;

    let data = if content.len() > 1024 {
        "Content can't have more than 1024 characters.".to_string()
    } else {
        let time = OffsetDateTime::now_utc()
            .format(&TIME_FORMAT.get().unwrap())
            .unwrap();
        let new_id = ctx.data().todo_data.get_id(ctx.channel_id());
        let nickname = match &assignee {
            Some(m) => get_member_nickname(m),
            None => "no one".to_string(),
        };
        let assignee = assignee.map(|m| m.user.id.0 as i64);

        let priority = priority.unwrap_or_default() as i32;

        let new_todo = NewTodo {
            channel_id: &(i64::from(ctx.channel_id())),
            id: &new_id,
            todo: &content,
            creation_date: &time,
            assignee,
            priority,
        };

        let result = diesel::insert_into(todos)
            .values(&new_todo)
            .execute(&mut ctx.data().db.get().unwrap());

        match result {
            Ok(_) => MessageBuilder::new()
                .push(format!("TODO [{}] (", &new_id))
                .push_mono_safe(&content)
                .push(format!(") added and assigned to {nickname}."))
                .build(),
            Err(NotFound) => "Not found.".to_string(),
            Err(_) => "Adding TODO failed.".to_string(),
        }
    };

    respond_text(ctx, data, false).await;

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
        Ok(deleted) => MessageBuilder::new()
            .push(format!("TODO [{}] (", &todo_id))
            .push_mono_safe(&deleted)
            .push(") deleted.")
            .build(),
        Err(NotFound) => "Not found.".to_string(),
        Err(_) => "Deleting TODO failed.".to_string(),
    };

    respond_text(ctx, data, true).await;

    Ok(())
}

/// Complete TODO entry
#[poise::command(slash_command)]
pub async fn complete(ctx: Context<'_>, #[description = "TODO id"] todo_id: i64) -> Result<()> {
    use crate::schema::todos::dsl::{channel_id, completion_date, id, todo, todos};

    let time = OffsetDateTime::now_utc()
        .format(&TIME_FORMAT.get().unwrap())
        .unwrap();

    let completed: QueryResult<String> = diesel::update(todos)
        .filter(channel_id.eq(i64::from(ctx.channel_id())))
        .filter(id.eq(todo_id as i32))
        .set(completion_date.eq(&time.to_string()))
        .returning(todo)
        .get_result(&mut ctx.data().db.get().unwrap());

    let data = match completed {
        Ok(completed) => MessageBuilder::new()
            .push(format!("TODO [{}] (", &todo_id))
            .push_mono_safe(&completed)
            .push(") completed.")
            .build(),
        Err(NotFound) => "Not found.".to_string(),
        Err(_) => "Completing TODO failed.".to_string(),
    };

    respond_text(ctx, data, false).await;

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
        Ok(uncompleted) => MessageBuilder::new()
            .push(format!("TODO [{}] (", &todo_id))
            .push_mono_safe(&uncompleted)
            .push(") uncompleted.")
            .build(),
        Err(NotFound) => "Not found.".to_string(),
        Err(_) => "Uncompleting TODO failed.".to_string(),
    };

    respond_text(ctx, data, true).await;

    Ok(())
}

/// Assign TODO entry
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
        Ok(reassigned) => MessageBuilder::new()
            .push(format!("TODO [{}] (", &todo_id))
            .push_mono_safe(&reassigned)
            .push(format!(") reassigned to {nickname}."))
            .build(),
        Err(NotFound) => "Not found.".to_string(),
        Err(_) => "Assigning TODO failed.".to_string(),
    };

    respond_text(ctx, data, true).await;

    Ok(())
}

/// Move TODO entry
#[poise::command(slash_command, rename = "move")]
pub async fn rmove(
    ctx: Context<'_>,
    #[description = "TODO id"] todo_id: i64,
    #[description = "TODO new channel"] new_channel: GuildChannel,
) -> Result<()> {
    use crate::schema::todos::dsl::{channel_id, id, todo, todos};

    let new_channel_id = new_channel.id.0 as i64;
    let new_id = ctx.data().todo_data.get_id(new_channel.id);

    let moved: QueryResult<String> = diesel::update(todos)
        .filter(channel_id.eq(i64::from(ctx.channel_id())))
        .filter(id.eq(todo_id as i32))
        .set((channel_id.eq(new_channel_id), id.eq(new_id)))
        .returning(todo)
        .get_result(&mut ctx.data().db.get().unwrap());

    let data = match moved {
        Ok(moved) => MessageBuilder::new()
            .push(format!("TODO [{}] (", &todo_id))
            .push_mono_safe(&moved)
            .push(format!(") moved to {}.", new_channel.name()))
            .build(),
        Err(NotFound) => "Not found.".to_string(),
        Err(_) => "Moving TODO failed.".to_string(),
    };

    respond_text(ctx, data, false).await;

    Ok(())
}

/// Edit TODO entry
#[poise::command(slash_command)]
pub async fn edit(
    ctx: Context<'_>,
    #[description = "TODO id"] todo_id: i64,
    #[description = "TODO new content"] content: String,
) -> Result<()> {
    use crate::schema::todos::dsl::{channel_id, id, todo, todos};

    let data = if content.len() > 1024 {
        "Content can't have more than 1024 characters.".to_string()
    } else {
        let text = content.replace('@', "@\u{200B}").replace('`', "'");

        let edited: QueryResult<String> = diesel::update(todos)
            .filter(channel_id.eq(i64::from(ctx.channel_id())))
            .filter(id.eq(todo_id as i32))
            .set(todo.eq(text))
            .returning(todo)
            .get_result(&mut ctx.data().db.get().unwrap());

        match edited {
            Ok(edited) => MessageBuilder::new()
                .push(format!("TODO [{}] edited to (", &todo_id))
                .push_mono_safe(&edited)
                .push(").".to_string())
                .build(),
            Err(NotFound) => "Not found.".to_string(),
            Err(_) => "Adding TODO failed.".to_string(),
        }
    };

    respond_text(ctx, data, true).await;

    Ok(())
}

/// Change priority of TODO entry
#[poise::command(slash_command)]
pub async fn set_priority(
    ctx: Context<'_>,
    #[description = "TODO id"] todo_id: i64,
    #[description = "TODO new priority"] new_priority: Option<Priority>,
) -> Result<()> {
    use crate::schema::todos::dsl::{channel_id, id, priority, todo, todos};

    let new_priority = new_priority.unwrap_or_default();
    let new_priority_int = new_priority as i32;

    let reassigned: QueryResult<String> = diesel::update(todos)
        .filter(channel_id.eq(i64::from(ctx.channel_id())))
        .filter(id.eq(todo_id as i32))
        .set(priority.eq(new_priority_int))
        .returning(todo)
        .get_result(&mut ctx.data().db.get().unwrap());

    let data = match reassigned {
        Ok(reassigned) => MessageBuilder::new()
            .push(format!("TODO [{}] (", &todo_id))
            .push_mono_safe(&reassigned)
            .push(format!(") new priority is {new_priority}."))
            .build(),

        Err(NotFound) => "Not found.".to_string(),
        Err(_) => "Assigning TODO failed.".to_string(),
    };

    respond_text(ctx, data, true).await;

    Ok(())
}

fn get_member_nickname(member: &Member) -> String {
    if let Some(nick) = &member.nick {
        return nick.to_string();
    }

    member.user.name.to_string()
}

#[derive(Debug, PartialEq)]
enum EmbedData {
    Text(String),
    Fields(Vec<TodoEntry>),
}

async fn respond_text(ctx: Context<'_>, text: String, ephemeral: bool) {
    let response = ctx
        .send(|reply| {
            reply
                .embed(|embed| embed.description(text))
                .ephemeral(ephemeral)
        })
        .await;

    if let Err(e) = response {
        debug!("{:?}", e);
    }
}
#[allow(clippy::too_many_lines)]
async fn respond_fields(ctx: Context<'_>, fields: Vec<TodoEntry>, query_data: QueryData) {
    let ctx_id = ctx.id();
    let prev_button_id = format!("{ctx_id}prev");
    let next_button_id = format!("{ctx_id}next");
    let refresh_button_id = format!("{ctx_id}refresh");

    let title = get_title(&query_data);
    let mut fields = fields;
    let mut page = 0;
    let mut pages = fields.len().div_ceil(DISCORD_EMBED_FIELDS_LIMIT as usize) as u32;

    let response = ctx
        .send(|reply| {
            reply.embed(|embed| {
                let footer = get_footer(&fields, page, pages);
                let fields = get_embed_data(&fields, page);
                embed
                    .title(&title)
                    .fields(fields)
                    .footer(|f| f.text(footer))
            });

            reply.components(|comp| {
                comp.create_action_row(|ar| {
                    ar.create_button(|cb| cb.custom_id(&prev_button_id).emoji('◀'))
                        .create_button(|cb| {
                            cb.custom_id(&refresh_button_id)
                                .label("Refresh")
                                .style(ButtonStyle::Secondary)
                        })
                        .create_button(|cb| cb.custom_id(&next_button_id).emoji('▶'))
                })
            });

            reply
        })
        .await;

    let mut message = match response {
        Ok(reply_handle) => reply_handle.into_message().await.unwrap(),
        Err(e) => {
            debug!("{:?}", e);
            return;
        }
    };

    while let Some(button) =
        poise::serenity_prelude::CollectComponentInteraction::new(ctx.serenity_context())
            .timeout(Duration::from_secs(60 * 30))
            .message_id(message.id)
            .filter(move |comp| comp.data.custom_id.starts_with(&ctx_id.to_string()))
            .await
    {
        let interaction_id = button.data.custom_id.clone();
        if interaction_id == prev_button_id {
            page = page.checked_sub(1).unwrap_or(pages - 1);
        } else if interaction_id == next_button_id {
            page += 1;
            if page >= pages {
                page = 0;
            }
        } else if interaction_id == refresh_button_id {
            let data = get_todos(ctx, &query_data).await;
            match data {
                EmbedData::Text(text) => {
                    let response = button
                        .create_interaction_response(ctx, |ir| {
                            ir.kind(poise::serenity_prelude::InteractionResponseType::UpdateMessage)
                                .interaction_response_data(|ird| {
                                    ird.embed(|ce| ce.description(text))
                                })
                        })
                        .await;

                    if let Err(e) = response {
                        debug!("{:?}", e);
                    }

                    continue;
                }
                EmbedData::Fields(embed_fields) => {
                    fields = embed_fields;
                    pages = fields.len().div_ceil(DISCORD_EMBED_FIELDS_LIMIT as usize) as u32;
                    page = 0;
                }
            }
        } else {
            continue;
        }

        let footer = get_footer(&fields, page, pages);
        let fields = get_embed_data(&fields, page);

        let response = button
            .create_interaction_response(ctx, |ir| {
                ir.kind(poise::serenity_prelude::InteractionResponseType::UpdateMessage)
                    .interaction_response_data(|ird| {
                        ird.embed(|ce| ce.title(&title).fields(fields).footer(|f| f.text(footer)))
                    })
            })
            .await;

        if let Err(e) = response {
            debug!("{:?}", e);
        }
    }

    page = 0;
    let footer = get_footer(&fields, page, pages);
    let fields = get_embed_data(&fields, page);

    let response = message
        .edit(ctx, |em| {
            em.embed(|ce| ce.title(title).fields(fields).footer(|f| f.text(footer)))
                .components(|cc| cc)
        })
        .await;

    if let Err(e) = response {
        debug!("{:?}", e);
    }
}

fn get_embed_data(fields: &[TodoEntry], page: u32) -> Vec<(String, String, bool)> {
    let skip = page * DISCORD_EMBED_FIELDS_LIMIT;
    let new_fields: Vec<(String, String, bool)> = fields
        .iter()
        .skip(skip.try_into().unwrap())
        .map(|entry| {
            let mut title = format!("[ {} ]", entry.id);
            if entry.priority != 0 {
                let priority = Priority::from(entry.priority);
                title = format!("{title} {priority}");
            }
            if entry.completed {
                title = format!("{title} [DONE]");
            }
            if let Some(nick) = &entry.assignee {
                title = format!("{title} - {nick}");
            };
            (title, entry.text.clone(), false)
        })
        .take(DISCORD_EMBED_FIELDS_LIMIT as usize)
        .collect();
    new_fields
}

fn get_footer(fields: &[TodoEntry], page: u32, pages: u32) -> String {
    let total = fields.iter().filter(|te| !te.completed).count();
    let footer = format!("Page {}/{pages}: {total} uncompleted TODOs", page + 1);
    footer
}

fn get_title(query_data: &QueryData) -> String {
    let mut title = String::from("TODOs");

    if let Some(assignee) = &query_data.todo_assignee {
        title = format!("{title} assigned to {}", assignee.user.name);
    }

    if query_data.completed {
        title = format!("{title} (w/ completed)");
    }

    if query_data.sort_by_priority {
        title = format!("{title} (sorted by priority)");
    }

    title
}
