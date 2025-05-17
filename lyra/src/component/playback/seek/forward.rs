use std::time::Duration;

use lyra_ext::pretty::duration_display::DurationDisplay;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{check, model::BotSlashCommand, require},
    core::model::response::initial::message::create::RespondWithMessage,
};

/// Seeks the current track forward to a new position some time later.
#[derive(CreateCommand, CommandModel)]
#[command(name = "forward")]
pub struct Forward {
    /// Seek by how many seconds? (If not given, 10 seconds)
    #[command(min_value = 0)]
    seconds: Option<f64>,
}

impl BotSlashCommand for Forward {
    async fn run(self, ctx: crate::command::SlashCtx) -> crate::error::CommandResult {
        let mut ctx = require::guild(ctx)?;
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let data_r = data.read().await;
        let queue = require::queue_not_empty(&data_r)?;
        let current_track = require::current_track(queue)?;
        check::current_track_is_users(&current_track, in_voice_with_user)?;

        let secs = self.seconds.unwrap_or(10.);
        if secs == 0. {
            ctx.wrng("Seconds must not be zero.").await?;
            return Ok(());
        }

        let old_timestamp = data_r.timestamp();
        let current_track_length = u128::from(current_track.track.data().info.length);
        drop(data_r);

        let timestamp = old_timestamp + Duration::from_secs_f64(secs);

        if timestamp.as_millis() > current_track_length {
            let remaining = timestamp.as_millis() - current_track_length;
            ctx.wrng(format!(
                "**Cannot seek past the end of the track**; Maximum forward seek is `{} seconds`.",
                remaining.div_ceil(1_000),
            ))
            .await?;
            return Ok(());
        }
        player
            .seek_to_with(timestamp, &mut data.write().await)
            .await?;

        ctx.out(format!(
            "⏩ ~~`{}`~~ ➜ **`{}`**.",
            old_timestamp.pretty_display(),
            timestamp.pretty_display(),
        ))
        .await?;
        Ok(())
    }
}
