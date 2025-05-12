use std::{collections::HashMap, sync::OnceLock};

use twilight_model::application::command::Command;

use crate::component::{
    config::Config,
    connection::{Join, Leave},
    controller::NowPlaying,
    misc::Ping,
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

        static SLASH_COMMAND_MAP: ::std::sync::LazyLock<SlashCommandMap> =
            ::std::sync::LazyLock::new(|| {
                ::paste::paste! {
                    SlashCommandMap {
                        $(
                            [<_ $raw_cmd:snake>]:
                                <$raw_cmd as ::twilight_interactions::command::CreateCommand>
                                    ::create_command()
                                    .into(),
                        )*
                    }
                }
            });

        type SlashCommandRefs = [
            &'static ::twilight_model::application::command::Command;
            SLASH_COMMANDS_N
        ];

        #[inline]
        fn slash_commands() -> SlashCommandRefs {
            ::paste::paste! {
                [
                    $(
                        &SLASH_COMMAND_MAP
                            .[<_ $raw_cmd:snake>],
                    )*
                ]
            }
        }

        $(
            impl $crate::command::model::ParentNameAware for $raw_cmd {
                const PARENT_NAME: ::core::option::Option<&'static str> =
                    ::core::option::Option::None;
            }
        )*

        type Callback = &'static (
            dyn ::std::ops::Fn(
                $crate::command::SlashCtx,
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

        static SLASH_COMMAND_CALLBACK:
            ::std::sync::LazyLock<
                ::std::collections::HashMap<&'static str, Callback>
            > = ::std::sync::LazyLock::new(|| {
                ::std::collections::HashMap::from(
                    ::paste::paste! {
                        [
                            $(
                                {
                                    #[::lavalink_rs::hook]
                                    async fn callback(
                                        ctx: $crate::command::SlashCtx,
                                        data: ::twilight_model::application::interaction::application_command::CommandData,
                                    ) -> ::std::result::Result<
                                        (),
                                        $crate::error::command::declare::CommandExecuteError,
                                    > {
                                        ::std::result::Result::Ok(
                                            $crate::command::model::BotSlashCommand::run(
                                                <$raw_cmd as ::twilight_interactions::command::CommandModel>
                                                    ::from_interaction(data.into())?,
                                                ctx
                                            ).await?
                                        )
                                    }

                                    (
                                        <$raw_cmd as ::twilight_interactions::command::CreateCommand>::NAME,
                                        &callback as Callback
                                    )
                                },
                            )*
                        ]
                    }
                )
            });

        impl $crate::command::SlashCtx {
            pub async fn execute(
                self,
                data: ::twilight_model::application::interaction::application_command::CommandData,
            ) -> ::std::result::Result<(), $crate::error::command::declare::CommandExecuteError> {
                $crate::command::check::user_allowed_in(&self).await?;

                if let ::std::option::Option::Some(callback) = SLASH_COMMAND_CALLBACK.get(&*data.name) {
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

        static MESSAGE_COMMAND_MAP: ::std::sync::LazyLock<MessageCommandMap> =
            ::std::sync::LazyLock::new(|| {
                ::paste::paste! {
                    MessageCommandMap {
                        $(
                            [<_ $raw_cmd:snake>]:
                                <$raw_cmd>::create_command().into(),
                        )*
                    }
                }
            });

        type MessageCommandRefs = [
            &'static ::twilight_model::application::command::Command;
            MESSAGE_COMMANDS_N
        ];

        #[inline]
        fn message_commands() -> MessageCommandRefs {
            ::paste::paste! {
                [
                    $(
                        &MESSAGE_COMMAND_MAP
                            .[<_ $raw_cmd:snake>],
                    )*
                ]
            }
        }

        impl $crate::command::MessageCtx {
            pub async fn execute(
                self,
                data: ::twilight_model::application::interaction::application_command::CommandData,
            ) -> ::std::result::Result<(), $crate::error::command::declare::CommandExecuteError> {
                $crate::command::check::user_allowed_in(&self).await?;

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

macro_rules! declare_autocomplete {
    ( $( $raw_cmd:ident => $raw_autocomplete:ident ),* $(,)? ) => {
        impl $crate::command::AutocompleteCtx {
            pub async fn execute(
                self,
                data: ::twilight_model::application::interaction::application_command::CommandData,
            ) -> ::std::result::Result<(), $crate::error::command::declare::AutocompleteExecuteError> {
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
    NowPlaying,
];

declare_message_commands![AddToQueue,];

declare_autocomplete![
    Play => PlayAutocomplete,
    Remove => RemoveAutocomplete,
    RemoveRange => RemoveRangeAutocomplete,
    Move => MoveAutocomplete,
    Jump => JumpAutocomplete,
];

pub static POPULATED_COMMAND_MAP: OnceLock<HashMap<&'static str, Command>> = OnceLock::new();

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
