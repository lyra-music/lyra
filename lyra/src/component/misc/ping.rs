use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::model::{BotGuildSlashCommand, BotSlashCommand, GuildSlashCmdCtx, SlashCmdCtx},
    core::model::response::initial::message::create::RespondWithMessage,
    error::CommandResult,
};

/// Shows the bot's latency.
#[derive(CreateCommand, CommandModel)]
#[command(name = "ping")]
pub struct Ping;

impl BotSlashCommand for Ping {
    async fn run(self, mut ctx: SlashCmdCtx) -> CommandResult {
        if let Some(latency) = ctx.latency().average() {
            ctx.out(format!("🏓 Pong! `({}ms)`", latency.as_millis()))
                .await?;
        } else {
            ctx.warn(
                "Cannot calculate the ping immediately after the bot has started, \
                try again shortly later.",
            )
            .await?;
        }
        Ok(())
    }
}

impl BotGuildSlashCommand for Ping {
    async fn run(self, ctx: GuildSlashCmdCtx) -> CommandResult {
        <Self as BotSlashCommand>::run(self, ctx.cast_as_non_guild()).await
    }
}
