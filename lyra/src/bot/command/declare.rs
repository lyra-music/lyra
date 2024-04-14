use std::{collections::HashMap, sync::OnceLock};

use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::application::{
    command::Command, interaction::application_command::CommandData,
};

use crate::bot::{
    command::{
        check,
        model::{BotAutocomplete, BotMessageCommand, BotSlashCommand, CommandInfoAware},
        AutocompleteCtx, MessageCtx, SlashCtx,
    },
    component::{
        config::Config,
        connection::{Join, Leave},
        misc::Ping,
        queue::{
            AddToQueue, Clear, FairQueue, Move, MoveAutocomplete, Play, PlayAutocomplete, PlayFile,
            Remove, RemoveAutocomplete, RemoveRange, RemoveRangeAutocomplete, Repeat, Shuffle,
        },
        tuning::Volume,
    },
    error::command::declare::{AutocompleteExecuteError, CommandExecuteError},
};

macro_rules! declare_slash_commands {
    ($( $raw_cmd: ident ),* $(,)? ) => {
        lazy_static::lazy_static! {
            static ref SLASH_COMMANDS_MAP: HashMap<Box<str>, Command> = HashMap::from([
                $(
                    (stringify!($raw_cmd).into(), <$raw_cmd>::create_command().into()),
                )*
            ]);
            pub static ref SLASH_COMMANDS: Box<[Command]> = SLASH_COMMANDS_MAP.clone().into_values().collect();
        }

        pub static POPULATED_COMMANDS_MAP: OnceLock<HashMap<Box<str>, Command>> = OnceLock::new();

        $(
            impl CommandInfoAware for $raw_cmd {
                fn name() -> &'static str {
                    let cmd_name = stringify!($raw_cmd);
                    &SLASH_COMMANDS_MAP.get(cmd_name)
                        .unwrap_or_else(|| panic!("command not found: {}", cmd_name))
                        .name
                }
            }
        )*

        impl SlashCtx {
            pub async fn execute(self, data: CommandData) -> Result<(), CommandExecuteError> {
                check::user_allowed_in(&self).await?;

                match data.name {
                    $(
                        ref n if n == <$raw_cmd>::name() => {
                            return Ok(<$raw_cmd>::from_interaction(data.into())?.run(self).await?);
                        }
                    )*
                    _ => {
                        let cmd_data = self.into_partial_command_data();
                        return Err(CommandExecuteError::UnknownCommand(cmd_data))
                    }
                }

            }
        }
    };
}

macro_rules! declare_message_commands {
    ($( $raw_cmd: ident ),* $(,)? ) => {
        lazy_static::lazy_static! {
            static ref MESSAGE_COMMANDS_MAP: HashMap<Box<str>, Command> = HashMap::from([
                $(
                    (stringify!($raw_cmd).into(), <$raw_cmd>::create_command().into()),
                )*
            ]);
            pub static ref MESSAGE_COMMANDS: Box<[Command]> = MESSAGE_COMMANDS_MAP.clone().into_values().collect();
        }

        $(
            impl CommandInfoAware for $raw_cmd {
                fn name() -> &'static str {
                    let cmd_name = stringify!($raw_cmd);
                    &MESSAGE_COMMANDS_MAP.get(cmd_name)
                        .unwrap_or_else(|| panic!("command not found: {}", cmd_name))
                        .name
                }
            }
        )*

        impl MessageCtx {
            pub async fn execute(self, data: CommandData) -> Result<(), CommandExecuteError> {
                check::user_allowed_in(&self).await?;

                match data.name {
                    $(
                        n if n == <$raw_cmd>::name() => {
                            return Ok(<$raw_cmd>::run(self).await?);
                        }
                    )*
                    _ => {
                        let cmd_data = self.into_partial_command_data();
                        return Err(CommandExecuteError::UnknownCommand(cmd_data))
                    }
                }

            }
        }
    };
}

macro_rules! declare_autocomplete {
    ($ ($raw_cmd: ident => $raw_autocomplete: ident) ,* $(,)? ) => {
        impl AutocompleteCtx {
            pub async fn execute(self, data: CommandData) -> Result<(), AutocompleteExecuteError> {
                match data.name {
                    $(
                        ref n if n == <$raw_cmd>::name() => {
                            return Ok(<$raw_autocomplete>::from_interaction(data.into())?.execute(self).await?);
                        }
                    )*
                    _ => {
                        let cmd_data = self.into_partial_command_data();
                        return Err(AutocompleteExecuteError::UnknownAutocomplete(cmd_data))
                    }
                }

            }
        }
    };
}

declare_slash_commands![
    Ping,
    Join,
    Leave,
    Config,
    Play,
    PlayFile,
    Repeat,
    Shuffle,
    FairQueue,
    Remove,
    RemoveRange,
    Clear,
    Move,
    Volume,
];
declare_message_commands![AddToQueue,];

declare_autocomplete![
    Play => PlayAutocomplete,
    Remove => RemoveAutocomplete,
    RemoveRange => RemoveRangeAutocomplete,
    Move => MoveAutocomplete,
];
