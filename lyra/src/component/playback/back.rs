use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::command::{check, macros::out, model::BotSlashCommand, require};

/// Jumps to the track before the current one in the queue. Will wrap around if queue repeat is enabled.
#[derive(CreateCommand, CommandModel)]
#[command(name = "back")]
pub struct Back;

impl BotSlashCommand for Back {
    #[allow(clippy::significant_drop_tightening)]
    async fn run(self, ctx: crate::command::SlashCtx) -> crate::error::CommandResult {
        let mut ctx = require::guild(ctx)?;
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let mut data_w = data.write().await;
        let queue = require::queue_not_empty_mut(&mut data_w)?;
        let mut txt;

        if let Ok(current_track) = require::current_track(queue) {
            check::current_track_is_users(&current_track, in_voice_with_user)?;
            txt = format!("⏮️ ~~`{}`~~", current_track.track.data().info.title);
        } else {
            txt = String::new();
        }

        queue.downgrade_repeat_mode();
        queue.acquire_advance_lock();
        queue.recede();

        // SAFETY: since the queue is not empty, receding must always yield a new current track
        let item = unsafe { queue.current().unwrap_unchecked() };
        player.context.play_now(item.data()).await?;

        if txt.is_empty() {
            txt = format!("⏮️ `{}`", item.data().info.title);
        }

        out!(txt, ctx);
    }
}
