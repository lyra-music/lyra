use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    LavalinkAndGuildIdAware,
    command::{SlashCmdCtx, model::BotSlashCommand, require},
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

impl BotSlashCommand for ToggleMute {
    async fn run(self, ctx: SlashCmdCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
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
