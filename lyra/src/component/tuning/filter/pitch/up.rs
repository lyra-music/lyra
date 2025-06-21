use std::num::NonZeroI64;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::model::{BotGuildSlashCommand, GuildSlashCmdCtx},
    component::tuning::{filter::pitch::shift_pitch, require_in_voice_unsuppressed_and_player},
    core::model::response::initial::message::create::RespondWithMessage,
    error::CommandResult,
};

/// Shifts the playback pitch up.
#[derive(CommandModel, CreateCommand)]
#[command(name = "up")]
pub struct Up {
    /// How many half tones? (If not given, 2)
    #[command(min_value = 1)]
    half_tones: Option<i64>,
}

impl BotGuildSlashCommand for Up {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let (_, player) = require_in_voice_unsuppressed_and_player(&ctx)?;

        let half_tones = NonZeroI64::new(self.half_tones.unwrap_or(2))
            .expect("half-tones step should be non-zero");
        let (old, new) = shift_pitch(&player, half_tones).await?;

        let emoji = new.tier().emoji();
        ctx.out(format!("{emoji}**`＋`** ~~`{old}`~~ ➜ **`{new}`**."))
            .await?;
        Ok(())
    }
}
