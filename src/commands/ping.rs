use serenity::model::interactions::application_command::ApplicationCommandInteraction;

use crate::commands::Command;


pub struct Ping;

impl Command for Ping {
    fn execute(_interaction: &ApplicationCommandInteraction) -> String {
        "Pong".to_string()
    }
}
