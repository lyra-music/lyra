use std::num::NonZeroUsize;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{
        check,
        macros::out,
        model::{BotSlashCommand, SlashCtx},
        require,
    },
    error::CommandResult,
    lavalink::{Event, LavalinkAware},
};

/// Clears the queue
#[derive(CommandModel, CreateCommand)]
#[command(name = "clear", dm_permission = false)]
pub struct Clear;

impl BotSlashCommand for Clear {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let in_voice = require::in_voice(&ctx)?.and_unsuppressed()?;
        let connection = ctx.lavalink().connection_from(&in_voice);
        let in_voice_with_user = check::in_voice_with_user(in_voice)?;
        let player = require::player(&ctx)?.and_queue_not_empty().await?;

        let data = player.data();
        {
            let data_r = data.read().await;
            let queue = data_r.queue();

            let positions = (1..=queue.len()).filter_map(NonZeroUsize::new);
            check::all_users_track(positions, in_voice_with_user, queue, &ctx)?;

            queue.stop_with_advance_lock(&player.context).await?;
            connection.dispatch(Event::QueueClear);
            drop(connection);
        }

        {
            let mut data_w = data.write().await;
            let queue = data_w.queue_mut();
            queue.clear();
        }
        out!("💥 Cleared the queue", ctx);
    }
}
