use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::command::{
    check,
    macros::{bad, out},
    model::BotSlashCommand,
    require,
};

/// Jumps to a new track at least two tracks earlier.
#[derive(CreateCommand, CommandModel)]
#[command(name = "backward")]
pub struct Backward {
    /// Jump by how many tracks?
    #[command(min_value = 2)]
    tracks: i64,
}

impl BotSlashCommand for Backward {
    #[allow(clippy::significant_drop_tightening)]
    async fn run(self, ctx: crate::command::SlashCtx) -> crate::error::CommandResult {
        let mut ctx = require::guild(ctx)?;
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let mut data_w = data.write().await;
        let queue = require::queue_not_empty_mut(&mut data_w)?;
        let current_track = require::current_track(queue)?;
        check::current_track_is_users(&current_track, in_voice_with_user)?;

        #[allow(clippy::cast_possible_truncation)]
        let tracks = self.tracks.unsigned_abs() as usize;
        let queue_index = queue.index();
        let Some(index) = queue_index.checked_sub(tracks) else {
            if queue_index == 0 {
                bad!("No where else to jump to", ctx);
            }
            bad!(
                format!(
                    "Cannot jump past the start of the queue. Maximum backward jump is {} tracks.",
                    queue_index,
                ),
                ctx
            );
        };

        queue.downgrade_repeat_mode();
        queue.acquire_advance_lock();

        let track = queue[index].data();
        let txt = format!("↩️ Jumped to `{}` (`#{}`)", track.info.title, index + 1);
        player.context.play_now(track).await?;

        *queue.index_mut() = index;
        out!(txt, ctx);
    }
}
