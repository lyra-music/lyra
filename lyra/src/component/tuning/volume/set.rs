use std::num::NonZeroU16;

use lyra_ext::num::cast::i64_as_u16;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::model::{BotGuildSlashCommand, GuildSlashCmdCtx},
    component::tuning::require_in_voice_unsuppressed_and_player,
    core::model::response::initial::message::create::RespondWithMessage,
    error::CommandResult,
};

/// Sets the playback volume.
#[derive(CommandModel, CreateCommand)]
#[command(name = "set")]
pub struct Set {
    /// Set the volume to what percentage? [1~1000%]
    #[command(min_value = 1, max_value = 1_000)]
    percent: i64,
}

impl BotGuildSlashCommand for Set {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let (_, player) = require_in_voice_unsuppressed_and_player(&ctx)?;

        let percent =
            NonZeroU16::new(i64_as_u16(self.percent)).expect("percent should be non-zero");
        player.context.set_volume(percent.get()).await?;
        player.data().write().await.set_volume(percent);

        let emoji = super::volume_emoji(Some(percent));
        let warning = super::clipping_warning(percent);

        ctx.out(format!("{emoji} `{percent}`%{warning}.")).await?;
        Ok(())
    }
}
