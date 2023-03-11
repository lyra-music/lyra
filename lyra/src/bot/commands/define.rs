use once_cell::sync::Lazy;
use twilight_interactions::command::CreateCommand;
use twilight_model::application::command::Command;

use super::models::LyraCommand;
use crate::bot::modules::misc::Ping;

macro_rules! define_commands {
    ($( $raw_cmd: ident ),* $(,)? ) => {
        static _COMMANDS_RAW: Lazy<Vec<Box<dyn LyraCommand>>> = Lazy::new(|| vec![
            $(
                Box::new($raw_cmd),
            )*
        ]);

        static _COMMANDS: Lazy<Vec<Command>> = Lazy::new(|| vec![
            $(
                <$raw_cmd>::create_command().into(),
            )*
        ]);
    };
}

pub static COMMANDS: Lazy<Vec<(&Box<dyn LyraCommand>, &Command)>> =
    Lazy::new(|| _COMMANDS_RAW.iter().zip(_COMMANDS.iter()).collect());

define_commands!(Ping);
