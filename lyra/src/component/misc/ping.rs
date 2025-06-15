use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::model::{
        BotGuildSlashCommand, BotSlashCommand, BotSlashCommand2, GuildSlashCmdCtx, SlashCmdCtx,
    },
    core::model::response::initial::message::create::RespondWithMessage,
    error::{
        CommandResult,
        component::misc::ping::{PingError, PingResidualError},
    },
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

impl BotSlashCommand2 for Ping {
    type Error = PingError;
    type ResidualError = PingResidualError;

    async fn run(self, ctx: &mut SlashCmdCtx) -> Result<(), Self::Error> {
        if let Some(latency) = ctx.latency().average() {
            ctx.out(format!("🏓 Pong! `({}ms)`", latency.as_millis()))
                .await?;
        } else {
            return Err(PingError::NoHeartbeatSent);
        }
        Ok(())
    }

    async fn handle_error(
        ctx: &mut SlashCmdCtx,
        error: Self::Error,
    ) -> Result<(), Self::ResidualError> {
        match error {
            PingError::Respond(e) => Err(e.into()),
            PingError::NoHeartbeatSent => {
                ctx.warn(
                    "Cannot calculate the ping immediately after the bot has started, \
                    try again shortly later.",
                )
                .await?;
                Ok(())
            }
        }
    }
}
