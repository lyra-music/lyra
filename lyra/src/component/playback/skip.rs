use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{
        check,
        model::{BotSlashCommand, GuildCtx, RespondViaMessage},
        require,
        util::controller_fmt,
    },
    core::model::response::initial::message::create::RespondWithMessage,
    error::component::playback::skip::SkipError,
    lavalink::OwnedPlayerData,
};

/// Skip playing the current track.
#[derive(CreateCommand, CommandModel)]
#[command(name = "skip", contexts = "guild")]
pub struct Skip;

impl BotSlashCommand for Skip {
    async fn run(self, ctx: crate::command::SlashCtx) -> crate::error::CommandResult {
        let mut ctx = require::guild(ctx)?;
        let player = require::player(&ctx)?;
        let data = player.data();
        require::queue_not_empty(&data.read().await)?;

        Ok(skip(player, data, &mut ctx, false).await?)
    }
}

pub async fn skip(
    player: require::PlayerInterface,
    data: OwnedPlayerData,
    ctx: &mut GuildCtx<impl RespondViaMessage>,
    via_controller: bool,
) -> Result<(), SkipError> {
    let data_r = data.read().await;
    let current_track = require::current_track(data_r.queue())?;
    let in_voice_with_user = check::user_in(require::in_voice(ctx)?.and_unsuppressed()?)?;

    let message = format!("⏭️ ~~`{}`~~.", current_track.track.data().info.title);

    // FAIRNESS: if a member requests to skip, it is fair to everyone in voice if the
    // current track is requested by that member as there will be no delays in upcoming
    // tracks.
    check::current_track_is_users(&current_track, in_voice_with_user)?;

    drop(data_r);

    let content = controller_fmt(ctx, via_controller, &message);
    ctx.out(content).await?;

    let mut data_w = data.write().await;
    let queue = data_w.queue_mut();
    queue.downgrade_repeat_mode();

    // CORRECTNESS: the current track is present in both scenarios:
    // - when called from `/skip`: verified via `queue_not_empty` and `current_track` checks
    // - when called from the skip button on the controller: if the controller exists, then
    //   it must only mean the current track also exists.
    // and will be ending via the `cleanup_now_playing_message_and_play` call later,
    // so this is correct.
    queue.disable_advancing();

    queue.advance();
    if let Some(index) = queue.mapped_index() {
        player
            .cleanup_now_playing_message_and_play(ctx, index, &mut data_w)
            .await?;
    } else {
        player
            .stop_and_delete_now_playing_message(&mut data_w)
            .await?;
    }
    drop(data_w);
    Ok(())
}
