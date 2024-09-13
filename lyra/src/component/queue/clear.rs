use std::num::NonZeroUsize;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{
        check,
        macros::out,
        model::{BotSlashCommand, SlashCtx},
        require,
    },
    error::CommandResult,
    gateway::GuildIdAware,
    lavalink::Event,
    LavalinkAware,
};

/// Clears the queue
#[derive(CommandModel, CreateCommand)]
#[command(name = "clear", dm_permission = false)]
pub struct Clear;

impl BotSlashCommand for Clear {
    #[allow(clippy::significant_drop_tightening)]
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let in_voice = require::in_voice(&ctx)?.and_unsuppressed()?;
        let connection = ctx.lavalink().try_get_connection(ctx.guild_id())?;
        let in_voice_with_user = check::user_in(in_voice)?;
        let player = require::player(&ctx)?;

        let data = player.data();
        {
            let data_r = data.read().await;
            let queue = require::queue_not_empty(&data_r)?;

            let positions = (1..=queue.len()).filter_map(NonZeroUsize::new);
            check::all_users_track(queue, positions, in_voice_with_user)?;

            player.acquire_advance_lock_and_stop_with(queue).await?;
            connection.dispatch(Event::QueueClear);
            drop(connection);
        }

        {
            let mut data_w = data.write().await;
            data_w.queue_mut().clear();
        }
        out!("⏹️ Cleared the queue", ctx);
    }
}
