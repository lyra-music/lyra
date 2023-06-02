use lyra_proc::err;
use twilight_mention::Mention;

use crate::bot::{
    commands::{
        checks::DJ_PERMISSIONS,
        declare::handle_commands,
        errors::{AlreadyInVoiceError, ConnectionError, Error},
        models::{App, Context},
    },
    inc::utils::BitFlagsPrettify,
};

pub async fn handle_app(ctx: Context<App>) -> anyhow::Result<()> {
    if let Err(e) = handle_commands(ctx.clone()).await {
        match e.downcast()? {
            Error::GuildOnly => {
                err!("❕ This command can only be used in guilds");
            }
            Error::Cache => {
                err!("⁉️ Something went wrong internally, please try again later");
            }
            Error::Connection {
                channel_id,
                source: ConnectionError::AlreadyInVoice(AlreadyInVoiceError::SomeoneElseInVoice),
            } => {
                err!(&format!("🚫 Someone else is using the bot in {}; You'll need **{}** permissions to do that", channel_id.mention(), DJ_PERMISSIONS.prettify()));
            }
            other => {
                return Err(other.into());
            }
        }
    };

    Ok(())
}
