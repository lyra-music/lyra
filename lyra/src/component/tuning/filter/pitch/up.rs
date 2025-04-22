use std::num::NonZeroI64;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{SlashCtx, macros::out, model::BotSlashCommand, require},
    component::tuning::{
        check_user_is_dj_and_require_unsuppressed_player, filter::pitch::shift_pitch,
    },
    error::CommandResult,
};

/// Shifts the playback pitch up
#[derive(CommandModel, CreateCommand)]
#[command(name = "up")]
pub struct Up {
    /// How many half tones? (If not given, 2)
    #[command(min_value = 1)]
    half_tones: Option<i64>,
}

impl BotSlashCommand for Up {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let (_, player) = check_user_is_dj_and_require_unsuppressed_player(&ctx)?;

        let half_tones = NonZeroI64::new(self.half_tones.unwrap_or(2))
            .expect("half-tones step should be non-zero");
        let (old, new) = shift_pitch(&player, half_tones).await?;

        let emoji = new.tier().emoji();
        out!(format!("{emoji}**`＋`** ~~`{old}`~~ ➜ **`{new}`**"), ctx);
    }
}
