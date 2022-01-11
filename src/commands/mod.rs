use serenity::model::interactions::application_command::ApplicationCommandInteraction;

pub trait Command {
    fn execute(interaction: &ApplicationCommandInteraction) -> String;
}

mod ping;
pub use ping::Ping;
