use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{macros::out, model::BotSlashCommand, require, SlashCtx},
    component::tuning::unmuting_checks,
    core::model::{BotStateAware, HttpAware},
    error::CommandResult,
    gateway::GuildIdAware,
    lavalink::LavalinkAware,
};

/// Toggles server muting the bot
#[derive(CommandModel, CreateCommand)]
#[command(name = "toggle-mute")]
pub struct ToggleMute;

impl BotSlashCommand for ToggleMute {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let in_voice = unmuting_checks(&ctx)?;

        let guild_id = ctx.guild_id();
        let mut connection = ctx.lavalink().connection_mut_from(&in_voice);

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
