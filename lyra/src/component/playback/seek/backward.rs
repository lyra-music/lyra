use std::time::Duration;

use lyra_ext::pretty::duration_display::DurationDisplay;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::command::{
    check,
    macros::{bad, out},
    model::BotSlashCommand,
    require,
};

/// Seeks the current track backward to a new position some time earlier.
#[derive(CreateCommand, CommandModel)]
#[command(name = "backward")]
pub struct Backward {
    /// Seek by how many seconds? (If not given, 5 seconds)
    #[command(min_value = 0)]
    seconds: Option<f64>,
}

impl BotSlashCommand for Backward {
    async fn run(self, ctx: crate::command::SlashCtx) -> crate::error::CommandResult {
        let mut ctx = require::guild(ctx)?;
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let data_r = data.read().await;
        let queue = require::queue_not_empty(&data_r)?;
        check::current_track_is_users(&require::current_track(queue)?, in_voice_with_user)?;

        let secs = self.seconds.unwrap_or(5.);
        if secs == 0. {
            bad!("Seconds can not be zero", ctx);
        }

        let old_timestamp = data_r.timestamp();
        drop(data_r);

        let timestamp = old_timestamp.saturating_sub(Duration::from_secs_f64(secs));
        player
            .seek_to_with(timestamp, &mut data.write().await)
            .await?;

        out!(
            format!(
                "⏪ ~~`{}`~~ ➜ **`{}`**",
                old_timestamp.pretty_display(),
                timestamp.pretty_display(),
            ),
            ctx
        );
    }
}
