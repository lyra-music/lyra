use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{
        check,
        macros::out,
        model::{BotSlashCommand, GuildCtx, RespondViaMessage},
        require,
        util::controller_fmt,
    },
    error::component::playback::PlayPauseError,
    lavalink::OwnedPlayerData,
};

/// Jumps to the track before the current one in the queue, wrapping around if queue repeat is enabled.
#[derive(CreateCommand, CommandModel)]
#[command(name = "back")]
pub struct Back;

impl BotSlashCommand for Back {
    async fn run(self, ctx: crate::command::SlashCtx) -> crate::error::CommandResult {
        let mut ctx = require::guild(ctx)?;
        let in_voice_with_user = check::user_in(require::in_voice(&ctx)?.and_unsuppressed()?)?;
        let player = require::player(&ctx)?;
        let data = player.data();

        let data_r = data.read().await;
        let queue = require::queue_not_empty(&data_r)?;
        let txt;

        if let Ok(current_track) = require::current_track(queue) {
            check::current_track_is_users(&current_track, in_voice_with_user)?;
            txt = Some(current_track.track.data().info.title.clone());
        } else {
            txt = None;
        }
        drop(data_r);

        Ok(back(txt, player, data, &mut ctx, false).await?)
    }
}

pub async fn back(
    current_track_title: Option<String>,
    player: require::PlayerInterface,
    data: OwnedPlayerData,
    ctx: &mut GuildCtx<impl RespondViaMessage>,
    via_controller: bool,
) -> Result<(), PlayPauseError> {
    let mut data_w = data.write().await;
    let queue = data_w.queue_mut();
    queue.downgrade_repeat_mode();
    queue.notify_skip_advance();
    queue.recede();
    let item = queue.current().expect("queue must be non-empty");
    player.context.play_now(item.data()).await?;
    let message = current_track_title.map_or_else(
        || format!("⏮️ `{}`.", item.data().info.title),
        |title| format!("⏮️ ~~`{title}`~~.",),
    );
    drop(data_w);

    let content = controller_fmt(ctx, via_controller, &message);
    out!(content, ctx);
}
