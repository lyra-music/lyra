use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::command::{check, macros::out, model::BotSlashCommand, require};

/// Skip playing the current track.
#[derive(CreateCommand, CommandModel)]
#[command(name = "skip")]
pub struct Skip;

impl BotSlashCommand for Skip {
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
        let txt = format!("⏭️ ~~`{}`~~", current_track.track.data().info.title);

        queue.downgrade_repeat_mode();
        queue.acquire_advance_lock();
        queue.advance();
        if let Some(item) = queue.current() {
            player.context.play_now(item.data()).await?;
        } else {
            player.context.stop_now().await?;
        }

        out!(txt, ctx);
    }
}
