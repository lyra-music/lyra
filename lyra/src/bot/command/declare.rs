use std::collections::HashMap;

use moka::future::Cache;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::{command::Command, interaction::application_command::CommandData},
    id::{marker::CommandMarker, Id},
};

use crate::bot::{
    command::{
        check,
        model::{
            AutocompleteCtx, BotAutocomplete, BotMessageCommand, BotSlashCommand, CommandInfoAware,
            MessageCommand, SlashCommand,
        },
        Ctx,
    },
    component::{
        config::Config,
        connection::{Join, Leave},
        misc::Ping,
        queue::{
            AddToQueue, Clear, FairQueue, Move, MoveAutocomplete, Play, PlayAutocomplete, PlayFile,
            Remove, RemoveAutocomplete, RemoveRange, RemoveRangeAutocomplete, Repeat, Shuffle,
        },
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

            pub static ref SLASH_COMMANDS_CACHE: Cache<Box<str>, Id<CommandMarker>> = Cache::new(100);
        }

        $(
            impl CommandInfoAware for $raw_cmd {
                fn name() -> Box<str> {
                    let cmd_name = stringify!($raw_cmd);
                    SLASH_COMMANDS_MAP.get(cmd_name)
                        .unwrap_or_else(|| panic!("command {} must exist", cmd_name))
                        .name
                        .clone()
                        .into()
                }
            }
        )*

        impl Ctx<SlashCommand> {
            pub async fn execute(mut self, data: CommandData) -> Result<(), CommandExecuteError> {
                check::user_allowed_in(&self).await?;

                // FIXME: update this to match statement once: https://github.com/rust-lang/rust/issues/86935
                $(
                    if <$raw_cmd>::name() == data.name.to_string().into() {
                        return Ok(<$raw_cmd>::from_interaction(data.into())?.run(self).await?);
                    }
                )*

                let cmd_data = self.take_partial_command_data().expect("`self.data` must exist");
                return Err(CommandExecuteError::UnknownCommand(cmd_data))
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

            pub static ref MESSAGE_COMMANDS_CACHE: Cache<Box<str>, Id<CommandMarker>> = Cache::new(5);
        }

        $(
            impl CommandInfoAware for $raw_cmd {
                fn name() -> Box<str> {
                    let cmd_name = stringify!($raw_cmd);
                    MESSAGE_COMMANDS_MAP.get(cmd_name)
                        .unwrap_or_else(|| panic!("command {} must exist", cmd_name))
                        .name
                        .clone()
                        .into()
                }
            }
        )*

        impl Ctx<MessageCommand> {
            pub async fn execute(mut self, data: CommandData) -> Result<(), CommandExecuteError> {
                check::user_allowed_in(&self).await?;

                // FIXME: update this to match statement once: https://github.com/rust-lang/rust/issues/86935
                $(
                    if <$raw_cmd>::name() == data.name.to_string().into() {
                        return Ok(<$raw_cmd>::run(self).await?);
                    }
                )*

                let cmd_data = self.take_partial_command_data().expect("`self.data` must exist");
                return Err(CommandExecuteError::UnknownCommand(cmd_data))
            }
        }
    };
}

macro_rules! declare_autocomplete {
    ($ ($raw_cmd: ident => $raw_autocomplete: ident) ,* $(,)? ) => {
        impl AutocompleteCtx {
            pub async fn execute(mut self, data: CommandData) -> Result<(), AutocompleteExecuteError> {
                // FIXME: update this to match statement once: https://github.com/rust-lang/rust/issues/86935
                $(
                    if <$raw_cmd>::name() == data.name.to_string().into() {
                        return Ok(<$raw_autocomplete>::from_interaction(data.into())?.execute(self).await?);
                    }
                )*

                let cmd_data = self.take_partial_command_data().expect("`self.data` must exist");
                return Err(AutocompleteExecuteError::UnknownAutocomplete(cmd_data))
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
];
declare_message_commands![AddToQueue,];

declare_autocomplete![
    Play => PlayAutocomplete,
    Remove => RemoveAutocomplete,
    RemoveRange => RemoveRangeAutocomplete,
    Move => MoveAutocomplete,
];
