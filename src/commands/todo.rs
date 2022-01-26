use std::{
    fmt,
    str::FromStr,
};

use serenity::model::interactions::application_command::{
    ApplicationCommandInteraction,
    ApplicationCommandInteractionDataOption,
    ApplicationCommandInteractionDataOptionValue as OptionValue,
    ApplicationCommandInteractionDataOptionValue::{
        String as OptionString,
        Integer as OptionInteger,
    },
};

use crate::{
    Handler,
    commands::Command
};


pub struct Todo;

#[derive(Debug)]
pub enum TodoActions {
    List,
    Add,
    Delete,
    Complete,
}

impl Todo {
    fn list(db: &sqlx::SqlitePool, channelid: u64) -> String {
        format!("todos on channel {}", channelid)
    }
    fn add(db: &sqlx::SqlitePool, channelid: u64, author: u64, text: String) -> String {
        format!("added todo on channel {}", channelid)
    }
    fn delete(db: &sqlx::SqlitePool, channelid: u64, id: u32) -> String {
        format!("deleted todo on channel {}", channelid)
    }
    fn complete(db: &sqlx::SqlitePool, channelid: u64, id: u32) -> String {
        format!("completed todo on channel {}", channelid)
    }
}

impl Command for Todo {
    fn execute(handler: &Handler, interaction: &ApplicationCommandInteraction) -> String {
        let options = &interaction.data.options.get(0).unwrap();
        println!("{:?}", options);
        let action = TodoActions::from_str(&options.name).unwrap();
        match action {
            TodoActions::List => Todo::list(&handler.db, interaction.channel_id.0),
            _ => {
                if !options.options.is_empty() {
                    let content = &options.options.get(0).unwrap().resolved;
                    println!("{:?} {:?}", action, content);
                    match action {
                        TodoActions::Add        => Todo::list(&handler.db, interaction.channel_id.0),
                        TodoActions::Delete     => Todo::list(&handler.db, interaction.channel_id.0),
                        TodoActions::Complete   => Todo::list(&handler.db, interaction.channel_id.0),
                        TodoActions::List       => unreachable!(),
                    }
                } else {
                    "Missing or incorrect argument".to_string()
                }
            }
        }
    }
}

impl fmt::Display for TodoActions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TodoActions::List       => write!(f, "list"),
            TodoActions::Add        => write!(f, "add"),
            TodoActions::Delete     => write!(f, "delete"),
            TodoActions::Complete   => write!(f, "complete"),
        }
    }
}

impl FromStr for TodoActions {

    type Err = ();

    fn from_str(input: &str) -> Result<TodoActions, Self::Err> {
        match input {
            "list"      => Ok(TodoActions::List),
            "add"       => Ok(TodoActions::Add),
            "delete"    => Ok(TodoActions::Delete),
            "complete"  => Ok(TodoActions::Complete),
            _           => Err(()),
        }
    }
}
