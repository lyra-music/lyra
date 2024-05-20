use std::num::NonZeroUsize;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{
        check,
        macros::out,
        model::{BotSlashCommand, SlashCtx},
    },
    error::CommandResult,
    gateway::ExpectedGuildIdAware,
    lavalink::{DelegateMethods, Event, LavalinkAware},
};

/// Clears the queue
#[derive(CommandModel, CreateCommand)]
#[command(name = "clear", dm_permission = false)]
pub struct Clear;

impl BotSlashCommand for Clear {
    async fn run(self, mut ctx: SlashCtx) -> CommandResult {
        let in_voice_with_user = check::in_voice(&ctx)?.with_user()?;
        check::queue_not_empty(&ctx).await?;
        check::not_suppressed(&ctx)?;

        let guild_id = ctx.guild_id();
        let lavalink = ctx.lavalink();

        let data = lavalink.player_data(guild_id);
        {
            let data_r = data.read().await;
            let queue = data_r.queue();

            let positions = (1..=queue.len()).filter_map(NonZeroUsize::new);
            check::all_users_track(positions, in_voice_with_user, queue, &ctx)?;

            queue.stop_with_advance_lock(guild_id, lavalink).await?;
            lavalink.dispatch(guild_id, Event::QueueClear);
        }

        {
            let mut data_w = data.write().await;
            let queue = data_w.queue_mut();
            queue.clear();
        }
        out!("💥 Cleared the queue", ctx);
    }
}
