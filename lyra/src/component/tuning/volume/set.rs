use std::num::NonZeroU16;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{SlashCmdCtx, model::BotSlashCommand, require},
    component::tuning::check_user_is_dj_and_require_unsuppressed_player,
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

impl BotSlashCommand for Set {
    async fn run(self, ctx: SlashCmdCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let (_, player) = check_user_is_dj_and_require_unsuppressed_player(&ctx)?;

        #[expect(clippy::cast_possible_truncation)]
        let percent = NonZeroU16::new(self.percent.unsigned_abs() as u16)
            .expect("percent should be non-zero");
        player.context.set_volume(percent.get()).await?;
        player.data().write().await.set_volume(percent);

        let emoji = super::volume_emoji(Some(percent));
        let warning = super::clipping_warning(percent);

        ctx.out(format!("{emoji} `{percent}`%{warning}.")).await?;
        Ok(())
    }
}
