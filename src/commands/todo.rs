use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI32, Ordering},
        Mutex,
    },
};

use chrono::{NaiveDateTime, Utc};
use diesel::{prelude::*, result::Error::NotFound};
use itertools::Itertools;
use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    client::Context,
    model::{
        application::{
            command::CommandOptionType,
            interaction::{
                application_command::{
                    ApplicationCommandInteraction,
                    CommandDataOptionValue::{
                        Boolean as OptBoolean, Integer as OptInteger, String as OptString,
                    },
                },
                InteractionResponseType,
            },
        },
        id::ChannelId,
    },
    utils::MessageBuilder,
};

use crate::{
    commands::{Command, CommandResult},
    models::*,
    secubot::{Conn, Secubot},
};

const TODO_COMMAND: &str = "todo";
const TODO_COMMAND_DESC: &str = "Todo";
const TODO_SUBCOMMAND_LIST: &str = "list";
const TODO_SUBCOMMAND_ADD: &str = "add";
const TODO_SUBCOMMAND_DELETE: &str = "delete";
const TODO_SUBCOMMAND_COMPLETE: &str = "complete";
const TODO_SUBCOMMAND_UNCOMPLETE: &str = "uncomplete";

type TodoEntry = (u64, String);

#[derive(Debug)]
enum TodoReturn {
    Text(String),
    Fields(Vec<TodoEntry>),
}

type TodoResult = Result<TodoReturn, String>;

#[derive(Debug)]
pub struct TodoCommand {
    iterators: Mutex<HashMap<ChannelId, AtomicI32>>,
    db: Conn,
}

impl TodoCommand {
    pub fn new(secubot: &Secubot) -> Self {
        use crate::schema::todos::dsl::*;

        let todo_list = todos.load::<Todo>(&mut secubot.db.get().unwrap()).unwrap();
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
            db: secubot.db.clone(),
        }
    }

    fn get_id(&self, channelid: ChannelId) -> i32 {
        let mut itr = self.iterators.lock().unwrap();
        let aint = itr.entry(channelid).or_insert_with(|| AtomicI32::new(0));
        aint.fetch_add(1, Ordering::SeqCst)
    }

    fn list(&self, channelid: ChannelId, completed: bool) -> TodoResult {
        use crate::schema::todos::dsl::*;

        // FIXME: looks bad, there needs to be smarter way
        let results = if completed {
            todos
                .filter(channel_id.eq(channelid.0 as i64))
                .load::<Todo>(&mut self.db.get().unwrap())
        } else {
            todos
                .filter(channel_id.eq(channelid.0 as i64))
                .filter(completion_date.is_null())
                .load::<Todo>(&mut self.db.get().unwrap())
        };

        match results {
            Ok(todo_list) => {
                let output: Vec<TodoEntry> = todo_list
                    .into_iter()
                    .map(|t| (t.id as u64, t.todo))
                    .collect();
                if output.is_empty() {
                    Ok(TodoReturn::Text(String::from(
                        "There are no incompleted TODOs in this channel.",
                    )))
                } else {
                    Ok(TodoReturn::Fields(output))
                }
            }
            Err(NotFound) => Err(String::from("Not found.")),
            Err(_) => Err(String::from("Listing TODOs failed.")),
        }
    }

    fn add(&self, channelid: ChannelId, text: &String) -> TodoResult {
        use crate::schema::todos::dsl::*;

        if text.len() > 1024 {
            Err(String::from(
                "Content can't have more than 1024 characters.",
            ))
        } else {
            let time = NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0);
            let new_id = self.get_id(channelid);
            let text = text.replace('@', "@\u{200B}").replace('`', "'");
            let new_todo = NewTodo {
                channel_id: &(channelid.0 as i64),
                id: &new_id,
                todo: &text,
                creation_date: &time.to_string(),
            };

            let result = diesel::insert_into(todos)
                .values(&new_todo)
                .execute(&mut self.db.get().unwrap());

            match result {
                Ok(_) => Ok(TodoReturn::Text(
                    MessageBuilder::new()
                        .push("TODO (")
                        .push_mono_safe(&text)
                        .push(") added.")
                        .build(),
                )),
                Err(NotFound) => Ok(TodoReturn::Text(String::from("Not found."))),
                Err(_) => Ok(TodoReturn::Text(String::from("Adding TODO failed."))),
            }
        }
    }

    fn delete(&self, _channelid: ChannelId, todo_id: i64) -> TodoResult {
        use crate::schema::todos::dsl::*;

        let deleted: Result<String, diesel::result::Error> = diesel::delete(todos)
            .filter(channel_id.eq(i64::from(_channelid)))
            .filter(id.eq(todo_id as i32))
            .returning(todo)
            .get_result(&mut self.db.get().unwrap());

        match deleted {
            Ok(deleted) => Ok(TodoReturn::Text(
                MessageBuilder::new()
                    .push("TODO (")
                    .push_mono_safe(&deleted)
                    .push(") deleted.")
                    .build(),
            )),
            Err(NotFound) => Ok(TodoReturn::Text(String::from("Not found."))),
            Err(_) => Ok(TodoReturn::Text(String::from("Deleting TODO failed."))),
        }
    }

    fn complete(&self, _channelid: ChannelId, todo_id: i64) -> TodoResult {
        use crate::schema::todos::dsl::*;

        let time = NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0);

        let completed: Result<String, diesel::result::Error> = diesel::update(todos)
            .filter(channel_id.eq(i64::from(_channelid)))
            .filter(id.eq(todo_id as i32))
            .set(completion_date.eq(&time.to_string()))
            .returning(todo)
            .get_result(&mut self.db.get().unwrap());

        match completed {
            Ok(completed) => Ok(TodoReturn::Text(
                MessageBuilder::new()
                    .push("TODO (")
                    .push_mono_safe(&completed)
                    .push(") completed.")
                    .build(),
            )),
            Err(NotFound) => Ok(TodoReturn::Text(String::from("Not found."))),
            Err(_) => Ok(TodoReturn::Text(String::from("Completing TODO failed."))),
        }
    }

    fn uncomplete(&self, _channelid: ChannelId, todo_id: i64) -> TodoResult {
        use crate::schema::todos::dsl::*;

        let uncompleted: Result<String, diesel::result::Error> = diesel::update(todos)
            .filter(channel_id.eq(i64::from(_channelid)))
            .filter(id.eq(todo_id as i32))
            .set(completion_date.eq::<Option<String>>(None))
            .returning(todo)
            .get_result(&mut self.db.get().unwrap());

        match uncompleted {
            Ok(uncompleted) => Ok(TodoReturn::Text(
                MessageBuilder::new()
                    .push("TODO (")
                    .push_mono_safe(&uncompleted)
                    .push(") uncompleted.")
                    .build(),
            )),
            Err(NotFound) => Ok(TodoReturn::Text(String::from("Not found."))),
            Err(_) => Ok(TodoReturn::Text(String::from("Uncompleting TODO failed."))),
        }
    }
}

#[async_trait]
impl Command for TodoCommand {
    fn get_name(&self) -> &'static str {
        TODO_COMMAND
    }

    fn add_application_command(&self, command: &mut CreateApplicationCommand) {
        command
            .description(TODO_COMMAND_DESC)
            .create_option(|option| {
                option
                    .name(TODO_SUBCOMMAND_LIST)
                    .description("List TODO entries")
                    .kind(CommandOptionType::SubCommand)
                    .create_sub_option(|subopt| {
                        subopt
                            .name("completed")
                            .description("Show completed TODOs")
                            .kind(CommandOptionType::Boolean)
                            .required(false)
                    })
            })
            .create_option(|option| {
                option
                    .name(TODO_SUBCOMMAND_ADD)
                    .description("Add TODO entry")
                    .kind(CommandOptionType::SubCommand)
                    .create_sub_option(|subopt| {
                        subopt
                            .name("content")
                            .description("TODO content")
                            .kind(CommandOptionType::String)
                            .required(true)
                    })
            })
            .create_option(|option| {
                option
                    .name(TODO_SUBCOMMAND_COMPLETE)
                    .description("Complete TODO entry")
                    .kind(CommandOptionType::SubCommand)
                    .create_sub_option(|subopt| {
                        subopt
                            .name("id")
                            .description("TODO id")
                            .kind(CommandOptionType::Integer)
                            .required(true)
                    })
            })
            .create_option(|option| {
                option
                    .name(TODO_SUBCOMMAND_UNCOMPLETE)
                    .description("Uncomplete TODO entry")
                    .kind(CommandOptionType::SubCommand)
                    .create_sub_option(|subopt| {
                        subopt
                            .name("id")
                            .description("TODO id")
                            .kind(CommandOptionType::Integer)
                            .required(true)
                    })
            })
            .create_option(|option| {
                option
                    .name(TODO_SUBCOMMAND_DELETE)
                    .description("Delete TODO entry")
                    .kind(CommandOptionType::SubCommand)
                    .create_sub_option(|subopt| {
                        subopt
                            .name("id")
                            .description("TODO id")
                            .kind(CommandOptionType::Integer)
                            .required(true)
                    })
            });
    }

    async fn handle(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
    ) -> CommandResult {
        let channel = command.channel_id;
        let subcommand = command
            .data
            .options
            .iter()
            .find(|x| x.kind == CommandOptionType::SubCommand)
            .unwrap();
        let subcommand_name = subcommand.name.as_str();
        let args = &subcommand.options;

        let result = match subcommand_name {
            TODO_SUBCOMMAND_LIST => {
                let completed = if let Some(opt) = args.iter().find(|x| x.name == "completed") {
                    if let OptBoolean(b) = opt.resolved.as_ref().unwrap() {
                        b
                    } else {
                        &false
                    }
                } else {
                    &false
                };
                self.list(channel, *completed)
            }
            TODO_SUBCOMMAND_ADD => {
                if let OptString(content) = args
                    .iter()
                    .find(|x| x.name == "content")
                    .expect("Expected content")
                    .resolved
                    .as_ref()
                    .expect("Expected content")
                {
                    self.add(channel, content)
                } else {
                    Err(String::from("Couldn't parse argument."))
                }
            }
            name => {
                if let OptInteger(id) = args
                    .iter()
                    .find(|x| x.name == "id")
                    .expect("Expected id")
                    .resolved
                    .as_ref()
                    .expect("Expected id")
                {
                    match name {
                        TODO_SUBCOMMAND_DELETE => self.delete(channel, *id),
                        TODO_SUBCOMMAND_COMPLETE => self.complete(channel, *id),
                        TODO_SUBCOMMAND_UNCOMPLETE => self.uncomplete(channel, *id),
                        &_ => {
                            unreachable! {}
                        }
                    }
                } else {
                    Err(String::from("Couldn't parse argument."))
                }
            }
        };

        let response_data = result.unwrap_or_else(|e| TodoReturn::Text(format!("ERROR: {}", e)));

        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.embed(|embed| match response_data {
                            TodoReturn::Text(text) => embed.description(text),
                            TodoReturn::Fields(fields) => {
                                let new_fields: Vec<(u64, String, bool)> = fields
                                    .into_iter()
                                    .map(|(x, y)| {
                                        let b = y.len() <= 25;
                                        (x, y, b)
                                    })
                                    .collect();
                                // new_fields.sort_by(|(_, _, x), (_, _, y)| y.cmp(x));
                                embed.title("TODOs").fields(new_fields)
                            }
                        })
                    })
            })
            .await?;

        Ok(())
    }
}
