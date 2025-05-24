use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{
        check,
        model::{BotGuildSlashCommand, GuildCtx, GuildSlashCmdCtx, RespondWithMessageKind},
        require,
        util::controller_fmt,
    },
    core::model::response::initial::message::create::RespondWithMessage,
    error::component::playback::PlayPauseError,
    lavalink::OwnedPlayerData,
};

/// Skip playing the current track.
#[derive(CreateCommand, CommandModel)]
#[command(name = "skip", contexts = "guild")]
pub struct Skip;

impl BotGuildSlashCommand for Skip {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> crate::error::CommandResult {
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let data_r = data.read().await;
        let queue = require::queue_not_empty(&data_r)?;
        let current_track = require::current_track(queue)?;
        check::current_track_is_users(&current_track, in_voice_with_user)?;
        let current_track_title = current_track.track.data().info.title.clone();
        drop(data_r);
        Ok(skip(&current_track_title, player, data, &mut ctx, false).await?)
    }
}

pub async fn skip(
    current_track_title: &str,
    player: require::PlayerInterface,
    data: OwnedPlayerData,
    ctx: &mut GuildCtx<impl RespondWithMessageKind>,
    via_controller: bool,
) -> Result<(), PlayPauseError> {
    let mut data_w = data.write().await;
    let queue = data_w.queue_mut();
    queue.downgrade_repeat_mode();
    queue.disable_advancing();
    queue.advance();
    if let Some(item) = queue.current() {
        player.context.play_now(item.data()).await?;
    } else {
        player.context.stop_now().await?;
    }
    drop(data_w);
    let message = format!("⏭️ ~~`{current_track_title}`~~.");
    let content = controller_fmt(ctx, via_controller, &message);
    ctx.out(content).await?;
    Ok(())
}
