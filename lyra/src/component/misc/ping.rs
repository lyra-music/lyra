use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{SlashCtx, model::BotSlashCommand},
    core::model::RespondWithMessage,
    error::CommandResult,
};

/// Shows the bot's latency.
#[derive(CreateCommand, CommandModel)]
#[command(name = "ping")]
pub struct Ping;

impl BotSlashCommand for Ping {
    async fn run(self, mut ctx: SlashCtx) -> CommandResult {
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
