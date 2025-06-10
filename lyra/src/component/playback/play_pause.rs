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
#[command(name = "play-pause", contexts = "guild")]
pub struct PlayPause;

impl BotSlashCommand for PlayPause {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let player = require::player(&ctx)?;
        let data = player.data();
        require::queue_not_empty(&data.read().await)?;

        Ok(play_pause(player, data, &mut ctx, false).await?)
    }
}

pub async fn play_pause(
    player: require::PlayerInterface,
    data: OwnedPlayerData,
    ctx: &mut GuildCtx<impl RespondViaMessage>,
    via_controller: bool,
) -> Result<(), PlayPauseError> {
    let in_voice_with_user = check::user_in(require::in_voice(ctx)?.and_unsuppressed()?)?;

    let data_r = data.read().await;
    let pause = !data_r.paused();
    if pause {
        // FAIRNESS: if a member requests to pause, they need to be the only person in voice,
        // as pausing will be unfair to everyone who queued after this current track: the
        // tracks after the current track will be delayed indefinitely until the player
        // unpaused.
        //
        // TODO: this only serves as a crude approximation, and it should be improved in the
        // future in a fairness rework of some sort. Ideally, if current track is `c`, then:
        // > forall track `x` after `c`: x.requester == c.requester
        in_voice_with_user.only()?;
    } else {
        // FAIRNESS: if a member requests to unpause, it is fair to everyone in voice if the
        // current track is requested by that member as there will be no delays in upcoming
        // tracks.
        check::current_track_is_users(
            &require::current_track(data_r.queue())?,
            in_voice_with_user,
        )?;
    }
    drop(data_r);

    let mut data_w = data.write().await;

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
