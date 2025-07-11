use std::{
    collections::HashMap,
    sync::{LazyLock, OnceLock},
};

use twilight_model::application::command::Command;

use crate::component::{
    config::Config,
    connection::{Join, Leave},
    controller::NowPlaying,
    misc::{Ping, Uptime},
    playback::{Back, Jump, JumpAutocomplete, PlayPause, Restart, Seek, Skip},
    queue::{
        AddToQueue, Clear, FairQueue, Move, MoveAutocomplete, Play, PlayAutocomplete, PlayFile,
        Remove, RemoveAutocomplete, RemoveRange, RemoveRangeAutocomplete, Repeat, Shuffle,
    },
    tuning::{Equaliser, Filter, Speed, Volume},
};

macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*));
}

macro_rules! declare_slash_commands {
    ( $( $raw_cmd:ident ),* $(,)? ) => {
        const SLASH_COMMANDS_N: usize = count!($($raw_cmd)*);

        ::paste::paste! {
            struct SlashCommandMap {
                $(
                    [<_ $raw_cmd:snake>]:
                        ::twilight_model::application::command::Command,
                )*
            }
        }

        fn slash_commands() -> [::twilight_model::application::command::Command; SLASH_COMMANDS_N] {
            // we can afford to initialise the entire map object without any memoisation,
            // as this will only be called once, in `command::declare::COMMANDS`.
            let map = ::paste::paste! {
                SlashCommandMap {
                    $(
                        [<_ $raw_cmd:snake>]:
                            <$raw_cmd as ::twilight_interactions::command::CreateCommand>
                                ::create_command()
                                .into(),
                    )*
                }
            };

            ::paste::paste!([$(map.[<_ $raw_cmd:snake>],)*])
        }

        $(
            impl $crate::command::model::CommandStructureAware for $raw_cmd {}
        )*
    };
}

macro_rules! declare_slash_commands_callback {
    ( $( $raw_cmd:ident ),* $(,)? ) => {
        impl $crate::command::model::SlashCmdCtx {
            pub async fn execute(
                self,
                data: ::twilight_model::application::interaction::application_command::CommandData,
            ) -> ::std::result::Result<(), $crate::error::command::declare::CommandExecuteError> {
                // as there aren't many non-guild slash commands,
                // matching each name branch by branch will have insignificant impact on runtime performance.
                match data.name {
                    $(
                        n if n ==
                            <$raw_cmd as ::twilight_interactions::command::CreateCommand>::NAME
                        => {
                            ::std::result::Result::Ok(<$raw_cmd as $crate::command::model::BotSlashCommand>::run($raw_cmd, self).await?)
                        }
                    )*
                    _ => {
                        let cmd_data = self.into_command_data();
                        ::std::result::Result::Err($crate::error::command::declare::CommandExecuteError::UnknownCommand(cmd_data))
                    }
                }
            }
        }
    }
}

macro_rules! declare_guild_slash_commands_callback {
    ( $( $raw_cmd:ident ),* $(,)? ) => {
        type GuildCallback = &'static (
            dyn ::std::ops::Fn(
                $crate::command::model::GuildSlashCmdCtx,
                ::twilight_model::application::interaction::application_command::CommandData,
            ) -> ::std::pin::Pin<::std::boxed::Box<
                    dyn ::std::future::Future<
                        Output = ::std::result::Result<
                            (),
                            $crate::error::command::declare::CommandExecuteError,
                        >
                    > + ::std::marker::Send
                >> + ::std::marker::Send + ::std::marker::Sync
        );

        static GUILD_SLASH_COMMAND_CALLBACK:
            // we cannot afford to initialise the entire map object without any memoisation,
            // as this will be called more than once: it will be called on every guild slash command execution.
            ::std::sync::LazyLock<
                ::std::collections::HashMap<&'static str, GuildCallback>
            > = ::std::sync::LazyLock::new(|| {
                ::std::collections::HashMap::from(
                    ::paste::paste! {
                        [
                            $(
                                {
                                    #[::lavalink_rs::hook]
                                    async fn callback(
                                        ctx: $crate::command::model::GuildSlashCmdCtx,
                                        data: ::twilight_model::application::interaction::application_command::CommandData,
                                    ) -> ::std::result::Result<
                                        (),
                                        $crate::error::command::declare::CommandExecuteError,
                                    > {
                                        ::std::result::Result::Ok(
                                            $crate::command::model::BotGuildSlashCommand::run(
                                                <$raw_cmd as ::twilight_interactions::command::CommandModel>
                                                    ::from_interaction(data.into())?,
                                                ctx
                                            ).await?
                                        )
                                    }

                                    (
                                        <$raw_cmd as ::twilight_interactions::command::CreateCommand>::NAME,
                                        &callback as GuildCallback
                                    )
                                },
                            )*
                        ]
                    }
                )
            });

        impl $crate::command::model::GuildSlashCmdCtx {
            pub async fn execute(
                self,
                data: ::twilight_model::application::interaction::application_command::CommandData,
            ) -> ::std::result::Result<(), $crate::error::command::declare::CommandExecuteError> {
                $crate::command::check::user_allowed_in(&self).await?;

                // as there are many guild slash commands,
                // matching each name branch by branch will have significant impact on runtime performance,
                // so a hash map of each command callbacks is used instead.
                if let ::std::option::Option::Some(callback) = GUILD_SLASH_COMMAND_CALLBACK.get(&*data.name) {
                    ::std::result::Result::Ok(callback(self, data).await?)
                } else {
                    let cmd_data = self.into_command_data();
                    ::std::result::Result::Err($crate::error::command::declare::CommandExecuteError::UnknownCommand(cmd_data))
                }
            }
        }
    };
}

macro_rules! declare_message_commands {
    ( $( $raw_cmd:ident ),* $(,)? ) => {
        const MESSAGE_COMMANDS_N: usize = count!($($raw_cmd)*);

        ::paste::paste! {
            struct MessageCommandMap {
                $(
                    [<_ $raw_cmd:snake>]:
                        ::twilight_model::application::command::Command,
                )*
            }
        }

        fn message_commands() -> [::twilight_model::application::command::Command; MESSAGE_COMMANDS_N] {
            // we can afford to initialise the entire map object without any memoisation,
            // as this will only be called once, in `command::declare::COMMANDS`.
            let map = ::paste::paste! {
                MessageCommandMap {
                    $(
                        [<_ $raw_cmd:snake>]:
                            <$raw_cmd>::create_command().into(),
                    )*
                }
            };

            ::paste::paste!([$(map.[<_ $raw_cmd:snake>],)*])
        }
    }
}

macro_rules! declare_message_commands_callback {
    ( $( $raw_cmd:ident ),* $(,)? ) => {
        impl $crate::command::model::MessageCmdCtx {
            pub async fn execute(
                self,
                data: ::twilight_model::application::interaction::application_command::CommandData,
            ) -> ::std::result::Result<(), $crate::error::command::declare::CommandExecuteError> {
                // as there aren't many non-guild message commands,
                // matching each name branch by branch will have insignificant impact on runtime performance.
                match data.name {
                    $(
                        n if n ==
                            <$raw_cmd as ::twilight_interactions::command::CreateCommand>::NAME
                        => {
                            ::std::result::Result::Ok(<$raw_cmd as $crate::command::model::BotMessageCommand>::run(self).await?)
                        }
                    )*
                    _ => {
                        let cmd_data = self.into_command_data();
                        ::std::result::Result::Err($crate::error::command::declare::CommandExecuteError::UnknownCommand(cmd_data))
                    }
                }
            }
        }
    };
}

macro_rules! declare_guild_message_commands_callback {
    ( $( $raw_cmd:ident ),* $(,)? ) => {
        impl $crate::command::model::GuildMessageCmdCtx {
            // as there aren't many guild message commands,
            // matching each name branch by branch will have insignificant impact on runtime performance.
            pub async fn execute(
                self,
                data: ::twilight_model::application::interaction::application_command::CommandData,
            ) -> ::std::result::Result<(), $crate::error::command::declare::CommandExecuteError> {
                $crate::command::check::user_allowed_in(&self).await?;

                match data.name {
                    $(
                        n if n ==
                            <$raw_cmd>::NAME
                        => {
                            ::std::result::Result::Ok(<$raw_cmd as $crate::command::model::BotGuildMessageCommand>::run(self).await?)
                        }
                    )*
                    _ => {
                        let cmd_data = self.into_command_data();
                        ::std::result::Result::Err($crate::error::command::declare::CommandExecuteError::UnknownCommand(cmd_data))
                    }
                }
            }
        }
    };
}

macro_rules! declare_autocomplete {
    ( $( $raw_cmd:ident => $raw_autocomplete:ident ),* $(,)? ) => {
        impl $crate::command::model::AutocompleteCtx {
            pub async fn execute(
                self,
                data: ::twilight_model::application::interaction::application_command::CommandData,
            ) -> ::std::result::Result<(), $crate::error::command::declare::AutocompleteExecuteError> {
                // as there aren't many non-guild slash commands with autocompletes,
                // matching each name branch by branch will have insignificant impact on runtime performance.
                match data.name {
                    $(
                        ref n if n ==
                            <$raw_cmd as ::twilight_interactions::command::CreateCommand>::NAME
                        => {
                            ::std::result::Result::Ok(
                                $crate::command::model::BotAutocomplete::execute(
                                    <$raw_autocomplete as ::twilight_interactions::command::CommandModel>
                                        ::from_interaction(data.into())?,
                                    self
                                ).await?
                            )
                        }
                    )*
                    _ => {
                        let cmd_data = self.into_command_data();
                        ::std::result::Result::Err($crate::error::command::declare::AutocompleteExecuteError::UnknownAutocomplete(cmd_data))
                    }
                }
            }
        }
    };
}

macro_rules! declare_guild_autocomplete {
    ( $( $raw_cmd:ident => $raw_autocomplete:ident ),* $(,)? ) => {
        impl $crate::command::model::GuildAutocompleteCtx {
            pub async fn execute(
                self,
                data: ::twilight_model::application::interaction::application_command::CommandData,
            ) -> ::std::result::Result<(), $crate::error::command::declare::AutocompleteExecuteError> {
                // as there aren't many guild slash commands with autocompletes,
                // matching each name branch by branch will have insignificant impact on runtime performance.
                match data.name {
                    $(
                        ref n if n ==
                            <$raw_cmd as ::twilight_interactions::command::CreateCommand>::NAME
                        => {
                            ::std::result::Result::Ok(
                                $crate::command::model::BotGuildAutocomplete::execute(
                                    <$raw_autocomplete as ::twilight_interactions::command::CommandModel>
                                        ::from_interaction(data.into())?,
                                    self
                                ).await?
                            )
                        }
                    )*
                    _ => {
                        let cmd_data = self.into_command_data();
                        ::std::result::Result::Err($crate::error::command::declare::AutocompleteExecuteError::UnknownAutocomplete(cmd_data))
                    }
                }
            }
        }
    };
}

declare_slash_commands![
    Ping,
    Uptime,
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
    NowPlaying,
];
declare_slash_commands_callback![Ping, Uptime];
declare_guild_slash_commands_callback![
    Ping,
    Uptime,
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
    NowPlaying,
];

declare_message_commands![AddToQueue,];
declare_message_commands_callback![];
declare_guild_message_commands_callback![AddToQueue,];

declare_autocomplete![];
declare_guild_autocomplete![
    Play => PlayAutocomplete,
    Remove => RemoveAutocomplete,
    RemoveRange => RemoveRangeAutocomplete,
    Move => MoveAutocomplete,
    Jump => JumpAutocomplete,
];

pub static POPULATED_COMMAND_MAP: OnceLock<HashMap<&'static str, Command>> = OnceLock::new();

const COMMANDS_N: usize = SLASH_COMMANDS_N + MESSAGE_COMMANDS_N;
type Commands = [Command; COMMANDS_N];

// we cannot afford to initialise the entire array object without any memoisation,
// as this will be called more than once: it will be called on every shard ready event.
pub static COMMANDS: LazyLock<Commands> = LazyLock::new(|| {
    let a = slash_commands();
    let b = message_commands();

    std::array::from_fn(|i| match i {
        0..SLASH_COMMANDS_N => a[i].clone(),
        SLASH_COMMANDS_N..COMMANDS_N => b[i - SLASH_COMMANDS_N].clone(),
        _ => unreachable!(),
    })
});
