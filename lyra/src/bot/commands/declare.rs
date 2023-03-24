use convert_case::{Case, Casing};
use once_cell::sync::Lazy;
use twilight_interactions::command::CommandModel;
use twilight_interactions::command::CreateCommand;
use twilight_model::application::command::Command;

use crate::bot::commands::models::App;
use crate::bot::commands::models::Context;
use crate::bot::commands::models::LyraCommand;
use crate::bot::modules::{connections::Join, misc::Ping};

macro_rules! replace_expr {
    ($_t:tt $sub:expr) => {
        $sub
    };
}

macro_rules! count_tts {
    ($( $tts: ident ),* $(,)? ) => {0usize $(+ replace_expr!($tts 1usize))*};
}

macro_rules! declare_commands {
    ($( $raw_cmd: ident ),* $(,)? ) => {
        pub static COMMANDS: Lazy<[Command; count_tts!($($raw_cmd, )*)]> = Lazy::new(|| [
            $(
                <$raw_cmd>::create_command().into(),
            )*
        ]);

        pub async fn handle_commands(ctx: Context<App>) -> anyhow::Result<()> {
            let cmd_data = ctx.command_data();

            match cmd_data.name.to_case(Case::Pascal).as_str() {
                $(
                    stringify!($raw_cmd) => Ok(<$raw_cmd>::from_interaction(cmd_data.into())?.callback(ctx).await?),
                )*
                unrecognized => Err(anyhow::anyhow!(format!("Unrecognized command: {}", unrecognized)))
            }
        }
    };

}

declare_commands!(Ping, Join);
