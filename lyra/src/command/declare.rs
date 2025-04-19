use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{LazyLock, OnceLock},
};

use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::application::{
    command::Command, interaction::application_command::CommandData,
};

use crate::{
    command::{
        AutocompleteCtx, MessageCtx, SlashCtx, check,
        model::{BotAutocomplete, BotMessageCommand, BotSlashCommand},
    },
    component::{
        config::Config,
        connection::{Join, Leave},
        misc::Ping,
        playback::{Back, Jump, JumpAutocomplete, PlayPause, Restart, Seek, Skip},
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
        const SLASH_COMMANDS_N: usize = count!($($raw_cmd)*);
        ::paste::paste! {
            struct SlashCommandMap {
                $([<_ $raw_cmd:snake>]: Command,)*
            }
        }

        static SLASH_COMMANDS_MAP: LazyLock<SlashCommandMap> = LazyLock::new(|| {
            ::paste::paste! {
                SlashCommandMap {
                    $([<_ $raw_cmd:snake>]: <$raw_cmd>::create_command().into(),)*
                }
            }
        });

        type SlashCommands = [Command; SLASH_COMMANDS_N];

        #[inline]
        fn slash_commands() -> SlashCommands {
            ::paste::paste! {
                [ $( SLASH_COMMANDS_MAP.[<_ $raw_cmd:snake>].clone(), )* ]
            }
        }

        // using a hashmap should significantly improve command respond times
        type Callback = &'static (dyn Fn(SlashCtx, CommandData) ->
            Pin<Box<dyn Future<Output = Result<(), CommandExecuteError>> + Send>> + Send + Sync);
        static SLASH_COMMANDS_CALLBACK: LazyLock<HashMap<&'static str, Callback>> = LazyLock::new(|| {
            HashMap::from(
                ::paste::paste! {[$({
                    #[lavalink_rs::hook]
                    async fn callback(ctx: SlashCtx, data: CommandData) -> Result<(), CommandExecuteError> {
                        Ok(<$raw_cmd>::from_interaction(data.into())?.run(ctx).await?)
                    }

                    ($raw_cmd::NAME, &callback as Callback)
                },)*]}
            )
        });

        impl SlashCtx {
            pub async fn execute(self, data: CommandData) -> Result<(), CommandExecuteError> {
                check::user_allowed_in(&self).await?;

                if let Some(callback) = SLASH_COMMANDS_CALLBACK.get(&*data.name) {
                    Ok(callback(self, data).await?)
                } else {
                    let cmd_data = self.into_command_data();
                    return Err(CommandExecuteError::UnknownCommand(cmd_data))
                }
            }
        }
    };
}

macro_rules! declare_message_commands {
    ($( $raw_cmd: ident ),* $(,)? ) => {
        const MESSAGE_COMMANDS_N: usize = count!($($raw_cmd)*);
        ::paste::paste! {
            struct MessageCommandMap {
                $([<_ $raw_cmd:snake>]: Command,)*
            }
        }

        static MESSAGE_COMMANDS_MAP: LazyLock<MessageCommandMap> = LazyLock::new(|| {
            ::paste::paste! {
                MessageCommandMap {
                    $([<_ $raw_cmd:snake>]: <$raw_cmd>::create_command().into(),)*
                }
            }
        });

        type MessageCommands = [Command; MESSAGE_COMMANDS_N];

        #[inline]
        fn message_commands() -> MessageCommands {
            ::paste::paste! {
                [ $( MESSAGE_COMMANDS_MAP.[<_ $raw_cmd:snake>].clone(), )* ]
            }
        }

        impl MessageCtx {
            pub async fn execute(self, data: CommandData) -> Result<(), CommandExecuteError> {
                check::user_allowed_in(&self).await?;

                // there aren't as much message commands, so this should be fast enough
                match data.name {
                    $(
                        n if n == <$raw_cmd>::NAME => {
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
                        ref n if n == <$raw_cmd>::NAME => {
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
    PlayPause,
    Seek,
    Restart,
    Jump,
    Skip,
    Back,
];

declare_message_commands![AddToQueue,];

declare_autocomplete![
    Play => PlayAutocomplete,
    Remove => RemoveAutocomplete,
    RemoveRange => RemoveRangeAutocomplete,
    Move => MoveAutocomplete,
    Jump => JumpAutocomplete,
];

pub static POPULATED_COMMANDS_MAP: OnceLock<HashMap<&'static str, Command>> = OnceLock::new();

const COMMANDS_N: usize = SLASH_COMMANDS_N + MESSAGE_COMMANDS_N;
type Commands = [Command; COMMANDS_N];

#[inline]
pub fn commands() -> Commands {
    let a = slash_commands();
    let b = message_commands();

    std::array::from_fn(|i| match i {
        0..SLASH_COMMANDS_N => a[i].clone(),
        SLASH_COMMANDS_N..COMMANDS_N => b[i - SLASH_COMMANDS_N].clone(),
        _ => unreachable!(),
    })
}
