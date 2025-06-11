use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{
        check,
        model::{BotGuildSlashCommand, GuildSlashCmdCtx},
        require,
    },
    core::model::response::initial::message::create::RespondWithMessage,
};

/// Jumps to the first track in the queue.
#[derive(CreateCommand, CommandModel)]
#[command(name = "first")]
pub struct First;

impl BotGuildSlashCommand for First {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> crate::error::CommandResult {
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let mut data_w = data.write().await;
        let queue = require::queue_not_empty_mut(&mut data_w)?;
        let current_track = require::current_track(queue);
        if let Ok(ref curr) = current_track {
            check::current_track_is_users(curr, in_voice_with_user)?;
        }
        let current_track_exists = current_track.is_ok();
        let queue_len = queue.len();
        if queue_len == 1 {
            ctx.wrng("No where else to jump to.").await?;
            return Ok(());
        }

        if queue.position().get() == 1 {
            ctx.wrng("Cannot jump to the current track.").await?;
            return Ok(());
        }

        queue.downgrade_repeat_mode();
        if current_track_exists {
            // CORRECTNESS: the current track is present and will be ending via the
            // `cleanup_now_playing_message_and_play` call later, so this is correct
            queue.disable_advancing();
        }

        let index = 0;
        let mapped_index = queue.map_index_expected(index);
        ctx.out(format!(
            "⬅️ Jumped to `{}` (`#{}`).",
            queue[mapped_index].data().info.title,
            mapped_index
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
