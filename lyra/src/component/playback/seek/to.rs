use std::time::Duration;

use lyra_ext::pretty::duration_display::{DurationDisplay, FromPrettyStr};
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{
        check,
        model::{BotGuildSlashCommand, GuildSlashCmdCtx},
        require,
    },
    component::playback::Restart,
    core::{
        http::InteractionClient, model::response::initial::message::create::RespondWithMessage,
    },
};

/// Seeks the current track to a new position.
#[derive(CreateCommand, CommandModel)]
#[command(name = "to")]
pub struct To {
    /// Seek to where? [Must be a timestamp like 1m23s or 4:56, or as the total seconds like 78s]
    #[command(min_length = 1)]
    timestamp: String,
}

impl BotGuildSlashCommand for To {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> crate::error::CommandResult {
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let data_r = data.read().await;
        let queue = require::queue_not_empty(&data_r)?;
        let current_track = require::current_track(queue)?;
        check::current_track_is_users(&current_track, in_voice_with_user)?;

        let timestamp_unchecked = self.timestamp;
        let timestamp = if let Ok(secs) = timestamp_unchecked.parse::<f64>() {
            if secs < 0. {
                ctx.wrng("Timestamp as total seconds must be positive.")
                    .await?;
                return Ok(());
            }

            Duration::from_secs_f64(secs)
        } else if let Ok(duration) = Duration::from_pretty_str(&timestamp_unchecked) {
            duration
        } else {
            ctx.wrng(
                format!(
                    "**Invalid timestamp: `{timestamp_unchecked}`**; \
                    Timestamp must either be in the format like `1m23s` or `4:56`, or as the total seconds like `78s`.",
                ),
            ).await?;
            return Ok(());
        };

        let current_track_length = u128::from(current_track.track.data().info.length);

        if timestamp.is_zero() {
            ctx.wrng(format!(
                "Timestamp must not be 0:00.\n\
                    -# To restart the track, use {} instead.",
                InteractionClient::mention_command::<Restart>()
            ))
            .await?;
            return Ok(());
        }
        if timestamp.as_millis() > current_track_length {
            ctx.wrng(format!(
                "**Invalid timestamp: `{}`**; Timestamp must be within the track length of `{}`.",
                timestamp.pretty_display(),
                current_track_length.pretty_display(),
            ))
            .await?;
            return Ok(());
        }

        let old_position = data_r.timestamp();
        drop(data_r);

        player
            .seek_to_with(timestamp, &mut data.write().await)
            .await?;
        ctx.out(format!(
            "🕹️ ~~`{}`~~ ➜ **`{}`**.",
            old_position.pretty_display(),
            timestamp.pretty_display(),
        ))
        .await?;
        Ok(())
    }
}
