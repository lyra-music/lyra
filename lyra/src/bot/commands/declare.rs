use std::collections::HashMap;

use anyhow::Result;
use once_cell::sync::Lazy;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::{
    application::command::Command,
    id::{marker::CommandMarker, Id},
};

use crate::bot::{
    commands::{
        checks,
        models::{App, LyraCommand, ResolvedCommandInfo},
        Context,
    },
    modules::{
        config::Config,
        connections::{Join, Leave},
        misc::Ping,
    },
};

macro_rules! declare_commands {
    ($( $raw_cmd: ident ),* $(,)? ) => {
        static COMMANDS_MAP: Lazy<HashMap<String, Command>> = Lazy::new(|| HashMap::from([
            $(
                (stringify!($raw_cmd).to_string(), <$raw_cmd>::create_command().into()),
            )*
        ]));

        pub static COMMANDS: Lazy<Vec<Command>> = Lazy::new(|| COMMANDS_MAP.clone().into_values().collect::<Vec<_>>());

        $(
            impl ResolvedCommandInfo for $raw_cmd {
                fn id() -> Id<CommandMarker> {
                    let cmd_name = stringify!($raw_cmd);
                    COMMANDS_MAP.get(cmd_name)
                        .unwrap_or_else(|| panic!("command {} must exist", cmd_name))
                        .id
                        .expect("`Command::id` must exist")
                }

                fn name() -> String {
                    let cmd_name = stringify!($raw_cmd);
                    COMMANDS_MAP.get(cmd_name)
                        .unwrap_or_else(|| panic!("command {} must exist", cmd_name))
                        .name
                        .clone()
                }
            }
        )*

        impl Context<App> {
            pub async fn execute(self: Context<App>) -> Result<()> {
                checks::user_allowed_in(&self).await?;

                let cmd_data = self.command_data().clone();

                // FIXME: update this to match statement once: https://github.com/rust-lang/rust/issues/86935
                $(
                    if cmd_data.name == <$raw_cmd>::name() {
                        return <$raw_cmd>::from_interaction(cmd_data.into())?.execute(self).await;
                    }
                )*
                Err(anyhow::anyhow!(format!("Unrecognized command: {}", cmd_data.name)))
            }
        }
    };
}

declare_commands!(Ping, Join, Leave, Config);
