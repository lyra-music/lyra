use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{check, macros::out, model::BotSlashCommand, SlashCtx},
    core::model::{BotStateAware, HttpAware},
    error::command::Result as CommandResult,
    gateway::ExpectedGuildIdAware,
    lavalink::LavalinkAware,
};

/// Toggles server muting the bot
#[derive(CommandModel, CreateCommand)]
#[command(name = "toggle-mute")]
pub struct ToggleMute;

impl BotSlashCommand for ToggleMute {
    async fn run(self, mut ctx: SlashCtx) -> CommandResult {
        check::user_is_dj(&ctx)?;
        check::in_voice(&ctx)?;

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
