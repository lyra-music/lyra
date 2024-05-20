use std::num::NonZeroI64;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{macros::out, model::BotSlashCommand, SlashCtx},
    component::tuning::{common_checks, filter::pitch::shift_pitch},
    error::CommandResult,
};

/// Shifts the playback pitch down
#[derive(CommandModel, CreateCommand)]
#[command(name = "down")]
pub struct Down {
    /// How many half tones? (If not given, 2)
    #[command(min_value = 1)]
    half_tones: Option<i64>,
}

impl BotSlashCommand for Down {
    async fn run(self, mut ctx: SlashCtx) -> CommandResult {
        common_checks(&ctx)?;

        let half_tones =
            NonZeroI64::new(self.half_tones.unwrap_or(2)).expect("self.half_tones is non-zero");
        let (old, new) = shift_pitch(&ctx, -half_tones).await?;

        let emoji = new.tier().emoji();
        out!(format!("{emoji}**`ー`** ~~`{old}`~~ ➜ **`{new}`**"), ctx);
    }
}
