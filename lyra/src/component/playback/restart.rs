use std::time::Duration;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{
        check,
        model::{BotGuildSlashCommand, GuildSlashCmdCtx},
        require,
    },
    core::model::response::initial::message::create::RespondWithMessage,
};

/// Restarts the current track; Equivalent to seeking to 0:00.
#[derive(CreateCommand, CommandModel)]
#[command(name = "restart", contexts = "guild")]
pub struct Restart;

impl BotGuildSlashCommand for Restart {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> crate::error::CommandResult {
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let data_r = data.read().await;
        let queue = require::queue_not_empty(&data_r)?;
        let current_track = require::current_track(queue)?;
        check::current_track_is_users(&current_track, in_voice_with_user)?;

        drop(data_r);
        player
            .seek_to_with(Duration::ZERO, &mut data.write().await)
            .await?;
        ctx.out("◀️ Restarted.").await?;
        Ok(())
    }
}
