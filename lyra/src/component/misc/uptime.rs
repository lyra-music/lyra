use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_mention::{
    Mention,
    timestamp::{Timestamp, TimestampStyle},
};

use crate::{
    command::model::{BotGuildSlashCommand, BotSlashCommand, GuildSlashCmdCtx, SlashCmdCtx},
    core::model::{BotStateAware, response::initial::message::create::RespondWithMessage},
    error::CommandResult,
};

/// Shows the bot's uptime.
#[derive(CreateCommand, CommandModel)]
#[command(name = "uptime")]
pub struct Uptime;

impl BotSlashCommand for Uptime {
    async fn run(self, mut ctx: SlashCmdCtx) -> CommandResult {
        let started = lyra_ext::unix_time() - ctx.bot().info().uptime();
        let stamp = Timestamp::new(started.as_secs(), Some(TimestampStyle::RelativeTime));
        ctx.out(format!("⏱️ {}.", stamp.mention())).await?;
        Ok(())
    }
}

impl BotGuildSlashCommand for Uptime {
    async fn run(self, ctx: GuildSlashCmdCtx) -> CommandResult {
        <Self as BotSlashCommand>::run(self, ctx.cast_as_non_guild()).await
    }
}
