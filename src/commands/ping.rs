use serenity::model::interactions::application_command::ApplicationCommandInteraction;

use crate::{
    Handler,
    commands::Command
};


pub struct Ping;

impl Command for Ping {
    fn execute(_handler: &Handler, _interaction: &ApplicationCommandInteraction) -> String {
        "Pong".to_string()
    }
}
