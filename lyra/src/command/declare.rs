use std::{collections::HashMap, sync::OnceLock};

use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::application::{
    command::Command, interaction::application_command::CommandData,
};

use crate::{
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
        tuning::{Equaliser, Filter, Speed, Volume},
    },
    error::command::declare::{AutocompleteExecuteError, CommandExecuteError},
};

macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*));
}

macro_rules! declare_slash_commands {
    ($( $raw_cmd: ident ),* $(,)? ) => {
        ::paste::paste! {
            struct SlashCommandMap {
                $([<_ $raw_cmd:snake>]: Command,)*
            }
        }

        fn slash_commands_map() -> &'static SlashCommandMap {
            static SLASH_COMMANDS_MAP: OnceLock<SlashCommandMap> = OnceLock::new();
            SLASH_COMMANDS_MAP.get_or_init(|| {
                ::paste::paste! {
                    SlashCommandMap {
                        $([<_ $raw_cmd:snake>]: <$raw_cmd>::create_command().into(),)*
                    }
                }
            })
        }

        type SlashCommands = [Command; count!($($raw_cmd)*)];

        pub fn slash_commands() -> &'static SlashCommands {
            static SLASH_COMMANDS: OnceLock<SlashCommands> = OnceLock::new();
            SLASH_COMMANDS.get_or_init(|| {
                let map = slash_commands_map();
                ::paste::paste! {
                    [$(map.[<_ $raw_cmd:snake>].clone(),)*]
                }
            })
        }

        pub static POPULATED_COMMANDS_MAP: OnceLock<HashMap<Box<str>, Command>> = OnceLock::new();

        $(
            impl CommandInfoAware for $raw_cmd {
                fn name() -> &'static str {
                    ::paste::paste! {
                        &slash_commands_map().[<_ $raw_cmd:snake>].name
                    }
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
                        let cmd_data = self.into_command_data();
                        return Err(CommandExecuteError::UnknownCommand(cmd_data))
                    }
                }
            }
        }
    };
}

macro_rules! declare_message_commands {
    ($( $raw_cmd: ident ),* $(,)? ) => {
        ::paste::paste! {
            struct MessageCommandMap {
                $([<_ $raw_cmd:snake>]: Command,)*
            }
        }

        fn message_commands_map() -> &'static MessageCommandMap {
            static MESSAGE_COMMANDS_MAP: OnceLock<MessageCommandMap> = OnceLock::new();
            MESSAGE_COMMANDS_MAP.get_or_init(|| {
                ::paste::paste! {
                    MessageCommandMap {
                        $([<_ $raw_cmd:snake>]: <$raw_cmd>::create_command().into(),)*
                    }
                }
            })
        }

        type MessageCommands = [Command; count!($($raw_cmd)*)];

        pub fn message_commands() -> &'static MessageCommands {
            static MESSAGE_COMMANDS: OnceLock<MessageCommands> = OnceLock::new();
            MESSAGE_COMMANDS.get_or_init(|| {
                let map = message_commands_map();
                ::paste::paste! {
                    [$(map.[<_ $raw_cmd:snake>].clone(),)*]
                }
            })
        }

        $(
            impl CommandInfoAware for $raw_cmd {
                fn name() -> &'static str {
                    ::paste::paste! {
                        &message_commands_map().[<_ $raw_cmd:snake>].name
                    }
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
                        let cmd_data = self.into_command_data();
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
                        let cmd_data = self.into_command_data();
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
    Filter,
    Speed,
    Equaliser,
];
declare_message_commands![AddToQueue,];

declare_autocomplete![
    Play => PlayAutocomplete,
    Remove => RemoveAutocomplete,
    RemoveRange => RemoveRangeAutocomplete,
    Move => MoveAutocomplete,
];
