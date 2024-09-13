use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{macros::out, model::BotSlashCommand, require, SlashCtx},
    component::tuning::unmuting_checks,
    core::model::{BotStateAware, HttpAware},
    error::CommandResult,
    gateway::GuildIdAware,
    LavalinkAware,
};

/// Toggles server muting the bot
#[derive(CommandModel, CreateCommand)]
#[command(name = "toggle-mute")]
pub struct ToggleMute;

impl BotSlashCommand for ToggleMute {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let _ = unmuting_checks(&ctx)?;

        let guild_id = ctx.guild_id();
        let mut connection = ctx.lavalink().try_get_connection_mut(guild_id)?;

        let mute = !connection.mute;
        ctx.http()
            .update_guild_member(guild_id, ctx.bot().user_id())
            .mute(mute)
            .await?;
        connection.mute = mute;
        drop(connection);

        let message = if mute { "🔇 Muted" } else { "🔊 Unmuted" };
        out!(message, ctx);
    }
}
