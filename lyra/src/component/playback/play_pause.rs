use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{
        SlashCtx, check,
        model::{BotSlashCommand, GuildCtx, RespondViaMessage},
        require,
        util::controller_fmt,
    },
    core::model::response::initial::message::create::RespondWithMessage,
    error::{CommandResult, component::playback::PlayPauseError},
    lavalink::OwnedPlayerData,
};

/// Toggles the playback of the current track.
#[derive(CreateCommand, CommandModel)]
#[command(name = "play-pause", dm_permission = false)]
pub struct PlayPause;

impl BotSlashCommand for PlayPause {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let data_r = data.read().await;
        let queue = require::queue_not_empty(&data_r)?;
        check::current_track_is_users(&require::current_track(queue)?, in_voice_with_user)?;
        drop(data_r);
        Ok(play_pause(player, data, &mut ctx, false).await?)
    }
}

pub async fn play_pause(
    player: require::PlayerInterface,
    data: OwnedPlayerData,
    ctx: &mut GuildCtx<impl RespondViaMessage>,
    via_controller: bool,
) -> Result<(), PlayPauseError> {
    let mut data_w = data.write().await;
    let pause = !data_w.paused();

    player.set_pause_with(pause, &mut data_w).await?;
    drop(data_w);

    let message = if pause {
        "▶️ Paused."
    } else {
        "⏸️ Resumed."
    };
    let content = controller_fmt(ctx, via_controller, message);
    ctx.out(content).await?;
    Ok(())
}
