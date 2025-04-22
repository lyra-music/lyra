use std::num::NonZeroU16;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{SlashCtx, macros::out, model::BotSlashCommand, require},
    component::tuning::check_user_is_dj_and_require_unsuppressed_player,
    error::CommandResult,
};

/// Set the playback volume
#[derive(CommandModel, CreateCommand)]
#[command(name = "set")]
pub struct Set {
    /// Set the volume to what percentage? [1~1000%]
    #[command(min_value = 1, max_value = 1_000)]
    percent: i64,
}

impl BotSlashCommand for Set {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let (_, player) = check_user_is_dj_and_require_unsuppressed_player(&ctx)?;

        #[allow(clippy::cast_possible_truncation)]
        let percent = NonZeroU16::new(self.percent.unsigned_abs() as u16)
            .expect("percent should be non-zero");
        player.context.set_volume(percent.get()).await?;
        player.data().write().await.set_volume(percent);

        let emoji = super::volume_emoji(Some(percent));
        let warning = super::clipping_warning(percent);

        out!(format!("{emoji} `{percent}`%{warning}"), ctx);
    }
}
