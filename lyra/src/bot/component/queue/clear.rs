use std::num::NonZeroUsize;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{
        check,
        macros::out,
        model::{BotSlashCommand, SlashCommand},
        Ctx,
    },
    error::command::Result as CommandResult,
    gateway::ExpectedGuildIdAware,
    lavalink::{ClientAware, Event},
};

/// Clears the queue
#[derive(CommandModel, CreateCommand)]
#[command(name = "clear", dm_permission = false)]
pub struct Clear;

impl BotSlashCommand for Clear {
    async fn run(self, mut ctx: Ctx<SlashCommand>) -> CommandResult {
        let in_voice_with_user = check::in_voice(&ctx)?.with_user()?;
        check::queue_not_empty(&ctx)?;
        check::not_suppressed(&ctx)?;

        let guild_id = ctx.guild_id_expected();
        let lavalink = ctx.lavalink();
        {
            let mut connection = lavalink.connection_mut(guild_id);
            let queue = connection.queue_mut();

            let positions = (1..=queue.len()).filter_map(NonZeroUsize::new);
            check::all_users_track(positions, in_voice_with_user, queue, &ctx)?;

            queue.stop_with_advance_lock(guild_id, lavalink).await?;

            connection.downgrade().dispatch(Event::QueueClear);
        }

        {
            let mut connection = lavalink.connection_mut(guild_id);
            let queue = connection.queue_mut();
            queue.clear();
        }
        out!("💥 Cleared the queue", ctx);
    }
}
