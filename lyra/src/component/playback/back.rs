use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{
        check,
        model::{BotSlashCommand, GuildCtx, RespondViaMessage},
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

impl BotSlashCommand for Back {
    async fn run(self, ctx: crate::command::SlashCtx) -> crate::error::CommandResult {
        let mut ctx = require::guild(ctx)?;
        let player = require::player(&ctx)?;
        let data = player.data();
        require::queue_not_empty(&data.read().await)?;

        Ok(back(player, data, &mut ctx, false).await?)
    }
}

pub async fn back(
    player: require::PlayerInterface,
    data: OwnedPlayerData,
    ctx: &mut GuildCtx<impl RespondViaMessage>,
    via_controller: bool,
) -> Result<(), BackError> {
    // FAIRNESS: if a member requests to back, they need to be the only person in voice,
    // as backing will be unfair to everyone who queued after this current track: the
    // tracks after the current track will be delayed for the track's duration.
    check::user_in(require::in_voice(ctx)?.and_unsuppressed()?)?.only()?;

    let current_track_title = data
        .read()
        .await
        .queue()
        .current()
        .map(|t| t.data().info.title.clone());

    let mut data_w = data.write().await;
    let queue = data_w.queue_mut();

    queue.downgrade_repeat_mode();
    if current_track_title.is_some() {
        // CORRECTNESS: the current track is present and will be ending via the
        // `cleanup_now_playing_message_and_play` call later, so this is correct.
        queue.disable_advancing();
    }
    queue.recede();

    let index = queue.mapped_index().expect("current track exists");
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
