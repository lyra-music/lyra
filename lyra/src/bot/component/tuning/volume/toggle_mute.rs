use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{macros::out, model::BotSlashCommand, SlashCtx},
    component::tuning::unmuting_checks,
    core::model::{BotStateAware, HttpAware},
    error::CommandResult,
    gateway::ExpectedGuildIdAware,
    lavalink::LavalinkAware,
};

/// Toggles server muting the bot
#[derive(CommandModel, CreateCommand)]
#[command(name = "toggle-mute")]
pub struct ToggleMute;

impl BotSlashCommand for ToggleMute {
    async fn run(self, mut ctx: SlashCtx) -> CommandResult {
        unmuting_checks(&ctx)?;

        let guild_id = ctx.guild_id();
        let mut connection = ctx.lavalink().connection_mut(guild_id);

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
