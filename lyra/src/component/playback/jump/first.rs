use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::command::{
    check,
    macros::{bad, out},
    model::BotSlashCommand,
    require,
};

/// Jumps to the first track in the queue
#[derive(CreateCommand, CommandModel)]
#[command(name = "first")]
pub struct First;

impl BotSlashCommand for First {
    async fn run(self, ctx: crate::command::SlashCtx) -> crate::error::CommandResult {
        let mut ctx = require::guild(ctx)?;
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let mut data_w = data.write().await;
        let queue = require::queue_not_empty_mut(&mut data_w)?;
        if let Ok(current_track) = require::current_track(queue) {
            check::current_track_is_users(&current_track, in_voice_with_user)?;
        }
        let queue_len = queue.len();
        if queue_len == 1 {
            bad!("No where else to jump to.", ctx);
        }

        if queue.position().get() == 1 {
            bad!("Cannot jump to the current track.", ctx);
        }

        queue.downgrade_repeat_mode();
        queue.notify_skip_advance();

        let track = queue[0].data();
        let txt = format!("⬅️ Jumped to `{}` (`#1`).", track.info.title);
        player.context.play_now(track).await?;

        *queue.index_mut() = 0;
        drop(data_w);
        out!(txt, ctx);
    }
}
