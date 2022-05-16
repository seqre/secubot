use async_trait::async_trait;

use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    model::{
        id::ChannelId,
        interactions::{
            application_command::{
                ApplicationCommandInteraction,
                ApplicationCommandInteractionDataOptionValue::Integer as OptInteger,
                ApplicationCommandInteractionDataOptionValue::String as OptString,
                ApplicationCommandOptionType,
            },
            InteractionResponseType,
        },
    },
};

use chrono::{NaiveDateTime, Utc};
use diesel::result::Error::NotFound;
use itertools::Itertools;

use std::{
    collections::HashMap,
    sync::atomic::{AtomicI32, Ordering},
};

use crate::{
    commands::{Command, CommandResult},
    models::*,
    secubot::{Conn, Secubot},
    *,
};

const TODO_COMMAND: &'static str = "todo";
const TODO_COMMAND_DESC: &'static str = "Todo";
const TODO_SUBCOMMAND_LIST: &'static str = "list";
const TODO_SUBCOMMAND_ADD: &'static str = "add";
const TODO_SUBCOMMAND_DELETE: &'static str = "delete";
const TODO_SUBCOMMAND_COMPLETE: &'static str = "complete";
const TODO_SUBCOMMAND_UNCOMPLETE: &'static str = "uncomplete";

type TodoEntry = (u64, String);

enum TodoReturn {
    Text(String),
    Fields(Vec<TodoEntry>),
}

type TodoResult = Result<TodoReturn, String>;

pub struct TodoCommand {
    iterators: Mutex<HashMap<ChannelId, AtomicI32>>,
}

impl TodoCommand {
    pub fn new(secubot: &Secubot) -> Self {
        use crate::schema::todos::dsl::*;

        let db = secubot.db.clone();
        let todo_list = todos.load::<Todo>(&*db.lock().unwrap()).unwrap();
        let iterators = todo_list
            .into_iter()
            .group_by(|td| td.channel_id)
            .into_iter()
            .map(|(chnl, tds)| {
                let biggest_id = match tds.map(|t| t.id).max() {
                    Some(b_id) => b_id,
                    None => 0,
                };
                (ChannelId(chnl as u64), AtomicI32::new(biggest_id + 1))
            })
            .collect::<HashMap<_, _>>();

        Self {
            iterators: Mutex::new(iterators),
        }
    }

    fn get_id(&self, channelid: ChannelId) -> i32 {
        let mut itr = self.iterators.lock().unwrap();
        let aint = itr.entry(channelid).or_insert_with(|| AtomicI32::new(0));
        aint.fetch_add(1, Ordering::SeqCst)
    }

    fn list(&self, db: &Conn, channelid: ChannelId) -> TodoResult {
        use crate::schema::todos::dsl::*;

        let results = todos
            .filter(channel_id.eq(channelid.0 as i64))
            .filter(completion_date.is_null())
            .load::<Todo>(&*db.lock().unwrap());

        match results {
            Ok(todo_list) => {
                let output: Vec<TodoEntry> = todo_list
                    .into_iter()
                    .map(|t| (t.id as u64, t.todo))
                    .collect();
                if output.is_empty() {
                    Ok(TodoReturn::Text(String::from(
                        "There are no incompleted TODOs in that channel.",
                    )))
                } else {
                    Ok(TodoReturn::Fields(output))
                }
            }
            Err(NotFound) => Ok(TodoReturn::Text(String::from("Not found."))),
            Err(_) => Ok(TodoReturn::Text(String::from("Err."))),
        }
    }

    fn add(&self, db: &Conn, channelid: ChannelId, text: String) -> TodoResult {
        use crate::schema::todos::dsl::*;

        if text.len() > 1024 {
            Err(String::from(
                "Content can't have more than 1024 characters.",
            ))
        } else {
            let time = NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0);
            let new_id = self.get_id(channelid);
            let new_todo = NewTodo {
                channel_id: &(channelid.0 as i64),
                id: &new_id,
                todo: &text,
                creation_date: &time.to_string(),
            };

            diesel::insert_into(todos)
                .values(&new_todo)
                .execute(&*db.lock().unwrap())
                .expect("Error while adding to database.");

            Ok(TodoReturn::Text(format!("TODO ``{}`` added.", &text)))
        }
    }

    fn delete(&self, db: &Conn, _channelid: ChannelId, todo_id: &i64) -> TodoResult {
        use crate::schema::todos::dsl::*;

        diesel::delete(todos.find(*todo_id as i32))
            .execute(&*db.lock().unwrap())
            .expect("Entry not found.");

        Ok(TodoReturn::Text(format!(
            "TODO (id: `{}`) deleted.",
            &todo_id
        )))
    }

    fn complete(&self, db: &Conn, _channelid: ChannelId, todo_id: &i64) -> TodoResult {
        use crate::schema::todos::dsl::*;

        let time = NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0);

        diesel::update(todos.find(*todo_id as i32))
            .set(completion_date.eq(&time.to_string()))
            .execute(&*db.lock().unwrap())
            .expect("Entry not found.");

        Ok(TodoReturn::Text(format!(
            "TODO (id: `{}`) completed.",
            &todo_id
        )))
    }
    fn uncomplete(&self, db: &Conn, _channelid: ChannelId, todo_id: &i64) -> TodoResult {
        use crate::schema::todos::dsl::*;

        diesel::update(todos.find(*todo_id as i32))
            .set(completion_date.eq::<Option<String>>(None))
            .execute(&*db.lock().unwrap())
            .expect("Entry not found.");

        Ok(TodoReturn::Text(format!(
            "TODO (id: `{}`) uncompleted.",
            &todo_id
        )))
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
                    .kind(ApplicationCommandOptionType::SubCommand)
            })
            .create_option(|option| {
                option
                    .name(TODO_SUBCOMMAND_ADD)
                    .description("Add TODO entry")
                    .kind(ApplicationCommandOptionType::SubCommand)
                    .create_sub_option(|subopt| {
                        subopt
                            .name("content")
                            .description("TODO content")
                            .kind(ApplicationCommandOptionType::String)
                            .required(true)
                    })
            })
            .create_option(|option| {
                option
                    .name(TODO_SUBCOMMAND_COMPLETE)
                    .description("Complete TODO entry")
                    .kind(ApplicationCommandOptionType::SubCommand)
                    .create_sub_option(|subopt| {
                        subopt
                            .name("id")
                            .description("TODO id")
                            .kind(ApplicationCommandOptionType::Integer)
                            .required(true)
                    })
            })
            .create_option(|option| {
                option
                    .name(TODO_SUBCOMMAND_UNCOMPLETE)
                    .description("Uncomplete TODO entry")
                    .kind(ApplicationCommandOptionType::SubCommand)
                    .create_sub_option(|subopt| {
                        subopt
                            .name("id")
                            .description("TODO id")
                            .kind(ApplicationCommandOptionType::Integer)
                            .required(true)
                    })
            })
            .create_option(|option| {
                option
                    .name(TODO_SUBCOMMAND_DELETE)
                    .description("Delete TODO entry")
                    .kind(ApplicationCommandOptionType::SubCommand)
                    .create_sub_option(|subopt| {
                        subopt
                            .name("id")
                            .description("TODO id")
                            .kind(ApplicationCommandOptionType::Integer)
                            .required(true)
                    })
            });
    }

    async fn handle(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
        secubot: &Secubot,
    ) -> CommandResult {
        let channel = command.channel_id;
        let subcommand = command
            .data
            .options
            .iter()
            .find(|x| x.kind == ApplicationCommandOptionType::SubCommand)
            .unwrap();
        let subcommand_name = subcommand.name.as_str();
        let args = &subcommand.options;

        println!("{:?}", subcommand);

        let result = match subcommand_name {
            TODO_SUBCOMMAND_LIST => self.list(&secubot.db.clone(), channel),
            TODO_SUBCOMMAND_ADD => {
                if let OptString(content) = args
                    .iter()
                    .find(|x| x.name == "content")
                    .expect("Expected content")
                    .resolved
                    .as_ref()
                    .expect("Expected content")
                {
                    self.add(&secubot.db.clone(), channel, String::from(content))
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
                        TODO_SUBCOMMAND_DELETE => self.delete(&secubot.db.clone(), channel, id),
                        TODO_SUBCOMMAND_COMPLETE => self.complete(&secubot.db.clone(), channel, id),
                        TODO_SUBCOMMAND_UNCOMPLETE => {
                            self.uncomplete(&secubot.db.clone(), channel, id)
                        }
                        &_ => {
                            unreachable! {}
                        }
                    }
                } else {
                    Err(String::from("Couldn't parse argument."))
                }
            }
        };

        let response_data = match result {
            Ok(content) => content,
            Err(error) => TodoReturn::Text(format!("ERROR: {}", error)),
        };

        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.embed(|embed| match response_data {
                            TodoReturn::Text(text) => embed.description(text),
                            TodoReturn::Fields(fields) => {
                                let mut new_fields: Vec<(u64, String, bool)> = fields
                                    .into_iter()
                                    .map(|(x, y)| {
                                        if y.len() > 25 {
                                            (x, y, false)
                                        } else {
                                            (x, y, true)
                                        }
                                    })
                                    .collect();
                                new_fields.sort_by(|(_, _, x), (_, _, y)| y.cmp(x));
                                embed.title("TODOs").fields(new_fields)
                            }
                        })
                    })
            })
            .await?;

        Ok(())
    }
}
