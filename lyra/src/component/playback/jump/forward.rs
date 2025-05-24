use std::num::NonZeroUsize;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{
        check,
        model::{BotGuildSlashCommand, GuildSlashCmdCtx},
        require,
    },
    core::model::response::initial::message::create::RespondWithMessage,
};

/// Jumps to a new track at least two tracks later.
#[derive(CreateCommand, CommandModel)]
#[command(name = "forward")]
pub struct Forward {
    /// Jump by how many tracks?
    #[command(min_value = 2)]
    tracks: i64,
}

impl BotGuildSlashCommand for Forward {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> crate::error::CommandResult {
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let mut data_w = data.write().await;
        let queue = require::queue_not_empty_mut(&mut data_w)?;
        let current_track = require::current_track(queue)?;

        #[expect(clippy::cast_possible_truncation)]
        let jump = self.tracks.unsigned_abs() as usize;
        let queue_len = queue.len();

        let queue_position = queue.position();
        let new_position = queue_position.saturating_add(jump);
        if new_position.get() > queue_len {
            let maximum_jump = queue_len - queue_position.get();
            if maximum_jump == 0 {
                ctx.wrng("No where else to jump to.").await?;
                return Ok(());
            }
            ctx.wrng(format!(
                "**Cannot jump past the end of the queue**; Maximum forward jump is `{maximum_jump} tracks`.",
            ))
            .await?;
            return Ok(());
        }

        let skipped =
            (current_track.position.get()..new_position.get()).filter_map(NonZeroUsize::new);
        check::all_users_track(queue, skipped, in_voice_with_user)?;

        queue.downgrade_repeat_mode();
        queue.disable_advancing();

        let track = queue[new_position].data();
        let txt = format!("↪️ Jumped to `{}` (`#{}`).", track.info.title, new_position);
        player.context.play_now(track).await?;

        *queue.index_mut() = new_position.get() - 1;
        drop(data_w);
        ctx.out(txt).await?;
        Ok(())
    }
}
