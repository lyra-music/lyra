use std::num::NonZeroUsize;

use lyra_ext::num::i64_as_usize;
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

        let jump = i64_as_usize(self.tracks);
        let queue_len = queue.len();

        let queue_position = queue.position();
        let new_position = queue_position.saturating_add(jump);
        let index = new_position.get() - 1;
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

        let skipped = (current_track.position.get()..=index).filter_map(NonZeroUsize::new);
        check::all_users_track(queue, skipped, in_voice_with_user)?;

        queue.downgrade_repeat_mode();

        // CORRECTNESS: the current track will always exist as this command cannot be used when the
        // current track doesn't exist, which is possible in two scenarios:
        // - queue is empty (which is impossible because of the `queue_not_empty_mut` check)
        // - the current queue index is past the end of the queue (which will early returned as
        //   "no where else to jump to"`)
        // the current track will be ending via the `cleanup_now_playing_message_and_play` call later,
        // so this is correct.
        queue.disable_advancing();

        let mapped_index = queue.map_index_expected(index);
        let track = queue[mapped_index].data();
        ctx.out(format!(
            "↪️ Jumped to `{}` (`#{}`).",
            track.info.title, mapped_index
        ))
        .await?;
        *queue.index_mut() = index;
        player
            .cleanup_now_playing_message_and_play(&ctx, mapped_index, &mut data_w)
            .await?;
        drop(data_w);

        Ok(())
    }
}
