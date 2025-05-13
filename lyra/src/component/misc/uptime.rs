use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_mention::{
    Mention,
    timestamp::{Timestamp, TimestampStyle},
};

use crate::{
    command::{SlashCtx, macros::out, model::BotSlashCommand},
    core::model::BotStateAware,
    error::CommandResult,
};

/// Shows the bot's uptime.
#[derive(CreateCommand, CommandModel)]
#[command(name = "uptime")]
pub struct Uptime;

impl BotSlashCommand for Uptime {
    async fn run(self, mut ctx: SlashCtx) -> CommandResult {
        let started = lyra_ext::unix_time() - ctx.bot().info().uptime();
        let stamp = Timestamp::new(started.as_secs(), Some(TimestampStyle::RelativeTime));
        out!(format!("⏱️ {}.", stamp.mention()), ctx);
    }
}
