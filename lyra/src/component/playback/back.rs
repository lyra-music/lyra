use std::num::NonZeroUsize;

use lyra_ext::num::range::nonzero_usize_range_inclusive;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{
        check,
        model::{BotGuildSlashCommand, GuildCtx, GuildSlashCmdCtx, RespondWithMessageKind},
        require,
        util::controller_fmt,
    },
    core::model::response::initial::message::create::RespondWithMessage,
    error::component::playback::back::BackError,
    lavalink::OwnedPlayerData,
};

/// Jumps to the track before the current one in the queue, wrapping around if queue repeat is enabled.
#[derive(CreateCommand, CommandModel)]
#[command(name = "back", contexts = "guild")]
pub struct Back;

impl BotGuildSlashCommand for Back {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> crate::error::CommandResult {
        let player = require::player(&ctx)?;
        let data = player.data();
        require::queue_not_empty(&data.read().await)?;

        Ok(back(player, data, &mut ctx, false).await?)
    }
}

pub async fn back(
    player: require::PlayerInterface,
    data: OwnedPlayerData,
    ctx: &mut GuildCtx<impl RespondWithMessageKind>,
    via_controller: bool,
) -> Result<(), BackError> {
    let in_voice_with_user = check::user_in(require::in_voice(ctx)?.and_unsuppressed()?)?;

    let data_r = data.read().await;
    let queue = data_r.queue();
    let (current, position) = queue.current_and_position();
    let current_track_title = current.map(|t| t.data().info.title.clone());

    if let Some(prev) = queue.prev_index() {
        let make = || NonZeroUsize::new(prev).expect("index + 1 must be non-zero");
        let positions =
            std::iter::once_with(make).chain(nonzero_usize_range_inclusive(position, queue.len()));
        check::users_tracks(queue, positions, in_voice_with_user)?;
    } else {
        check::users_tracks_from(queue, position, in_voice_with_user)?;
    }
    drop(data_r);

    let mut data_w = data.write().await;
    let queue = data_w.queue_mut();

    queue.downgrade_repeat_mode();
    if current_track_title.is_some() {
        // CORRECTNESS: the current track is present and will be ending via the
        // `cleanup_now_playing_message_and_play` call later, so this is correct.
        queue.disable_advancing();
    }
    queue.recede();

    let index = queue.current_index().expect("current track exists");
    let message = current_track_title.map_or_else(
        || format!("⏮️ `{}`.", queue[index].data().info.title),
        |title| format!("⏮️ ~~`{title}`~~.",),
    );
    ctx.out(controller_fmt(ctx, via_controller, &message))
        .await?;

    player
        .cleanup_now_playing_message_and_play(ctx, index, &mut data_w)
        .await?;

    drop(data_w);
    Ok(())
}
