use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::model::GuildSlashCmdCtx,
    component::tuning::{UpdateFilter, require_in_voice_unsuppressed_and_player},
    core::model::response::initial::message::create::RespondWithMessage,
};

/// Disables the player equaliser.
#[derive(CommandModel, CreateCommand)]
#[command(name = "off")]
pub struct Off;

impl crate::command::model::BotGuildSlashCommand for Off {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> crate::error::CommandResult {
        let (_, player) = require_in_voice_unsuppressed_and_player(&ctx)?;

        player.update_filter(None::<super::SetEqualiser>).await?;
        ctx.out("🎛️🔴 Disabled equaliser.").await?;
        Ok(())
    }
}
