use serenity::model::interactions::application_command::ApplicationCommandInteraction;

use crate::Handler;

pub trait Command {
    fn execute(handler: &Handler, interaction: &ApplicationCommandInteraction) -> String;
}

//use std::collections::HashMap;

//static COMMANDS: HashMap<String, Box<dyn Command>> = HashMap::from([
//    ("ping".to_string(), Box::new(ping::Ping))
//]);


mod ping;
pub use ping::Ping;

mod todo;
pub use todo::{Todo, TodoActions};
