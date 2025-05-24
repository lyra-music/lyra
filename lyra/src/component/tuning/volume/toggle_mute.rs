use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    LavalinkAndGuildIdAware,
    command::model::{BotGuildSlashCommand, GuildSlashCmdCtx},
    component::tuning::unmuting_checks,
    core::model::{
        BotStateAware, HttpAware, response::initial::message::create::RespondWithMessage,
    },
    error::CommandResult,
    gateway::GuildIdAware,
};

/// Toggles server muting the bot.
#[derive(CommandModel, CreateCommand)]
#[command(name = "toggle-mute")]
pub struct ToggleMute;

impl BotGuildSlashCommand for ToggleMute {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let _ = unmuting_checks(&ctx)?;

        let guild_id = ctx.guild_id();
        let mute = ctx.get_conn().toggle_mute().await?;
        ctx.http()
            .update_guild_member(guild_id, ctx.bot().user_id())
            .mute(mute)
            .await?;

        let message = if mute { "🔇 Muted." } else { "🔊 Unmuted." };
        ctx.out(message).await?;
        Ok(())
    }
}
