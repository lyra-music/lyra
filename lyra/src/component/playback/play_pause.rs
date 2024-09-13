use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{check, macros::out, model::BotSlashCommand, require, SlashCtx},
    error::CommandResult,
};

/// Toggles the playback of the current track.
#[derive(CreateCommand, CommandModel)]
#[command(name = "play-pause", dm_permission = false)]
pub struct PlayPause;

impl BotSlashCommand for PlayPause {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let data_r = data.read().await;
        let queue = require::queue_not_empty(&data_r)?;
        check::current_track_is_users(&require::current_track(queue)?, in_voice_with_user)?;
        let pause = !data_r.paused();
        let message = if pause {
            "▶️ Paused"
        } else {
            "⏸️ Resumed"
        };
        drop(data_r);

        player
            .set_pause_with(pause, &mut data.write().await)
            .await?;

        out!(message, ctx);
    }
}
