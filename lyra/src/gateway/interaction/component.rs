use tokio::sync::oneshot;
use twilight_model::application::interaction::InteractionData;

use crate::{
    command::{
        model::{ComponentCtx, ComponentMarker, GuildCtx},
        require,
    },
    core::{
        model::{ctx_head::CtxHead, response::Respond},
        r#static::component::NowPlayingButtonType,
    },
    error::gateway::{
        ProcessError, ProcessResult,
        component::{ControllerError, Fe},
    },
    lavalink::OwnedPlayerData,
};

use super::{
    match_cache, match_in_voice_with_someone_else, match_in_voice_without_user, match_lavalink,
    match_not_in_voice, match_not_playing, match_not_users_track, match_suppressed,
    match_unrecognised_connection, match_wildcard,
};

impl super::Context {
    pub(super) async fn process_as_component(mut self) -> ProcessResult {
        let bot = self.bot;
        let mut i = bot.interaction().ctx(&self.inner);
        let Some(InteractionData::MessageComponent(data)) = self.inner.data.take() else {
            unreachable!()
        };
        tracing::trace!(?data);

        let (tx, mut rx) = oneshot::channel::<()>();
        let ctx = ComponentCtx::from_data(self.inner, data, bot, self.latency, self.sender, tx);
        let Ok(mut ctx) = require::guild(ctx) else {
            return Ok(());
        };
        let Ok(player) = require::player(&ctx) else {
            return Ok(());
        };

        let player_data = player.data();
        let player_data_r = player_data.read().await;
        let now_playing_message_id = player_data_r.now_playing_message_id();
        if now_playing_message_id.is_none_or(|id| id != ctx.message().id) {
            return Ok(());
        }
        drop(player_data_r);

        let Some(now_playing_button) = ctx.take_custom_id_into_now_playing_button_type() else {
            return Ok(());
        };

        let Err(error) = execute_controller(ctx, player, player_data, now_playing_button).await
        else {
            return Ok(());
        };

        if rx.try_recv().is_ok() {
            i.acknowledge();
        }
        match_error(error, now_playing_button, i).await
    }
}

async fn execute_controller(
    mut ctx: GuildCtx<ComponentMarker>,
    player: require::PlayerInterface,
    player_data: OwnedPlayerData,
    now_playing_button: NowPlayingButtonType,
) -> Result<(), ControllerError> {
    match now_playing_button {
        NowPlayingButtonType::Shuffle => {
            crate::component::queue::shuffle(player_data.clone(), &mut ctx, true).await?;
        }
        NowPlayingButtonType::Previous => {
            crate::component::playback::back(player, player_data.clone(), &mut ctx, true).await?;
        }
        NowPlayingButtonType::PlayPause => {
            crate::component::playback::play_pause(player, player_data.clone(), &mut ctx, true)
                .await?;
        }
        NowPlayingButtonType::Next => {
            crate::component::playback::skip(player, player_data.clone(), &mut ctx, true).await?;
        }
        NowPlayingButtonType::Repeat => {
            let mode = crate::component::queue::get_next_repeat_mode(&ctx).await;
            crate::component::queue::repeat(&mut ctx, player_data.clone(), mode, true).await?;
        }
    }
    Ok(())
}

async fn match_error(
    error: ControllerError,
    now_playing_button: NowPlayingButtonType,
    mut i: CtxHead,
) -> ProcessResult {
    match error.flatten_as() {
        Fe::Cache => Ok(match_cache(error, i).await?),
        Fe::InVoiceWithoutUser(e) => Ok(match_in_voice_without_user(e, i).await?),
        Fe::Suppressed(e) => Ok(match_suppressed(e, i).await?),
        Fe::Lavalink(_) => Ok(match_lavalink(
            error,
            |e| ProcessError::ControllerExecute {
                kind: now_playing_button,
                source: e,
            },
            &mut i,
        )
        .await?),
        Fe::NotInVoice => Ok(match_not_in_voice(i).await?),
        Fe::InVoiceWithSomeoneElse(e) => Ok(match_in_voice_with_someone_else(e, i).await?),
        Fe::NotPlaying => Ok(match_not_playing(i).await?),
        Fe::UnrecognisedConnection => Ok(match_unrecognised_connection(i).await?),
        Fe::NotUsersTrack(e) => Ok(match_not_users_track(e, i).await?),
        _ => Ok(match_wildcard(
            error,
            |e| ProcessError::ControllerExecute {
                kind: now_playing_button,
                source: e,
            },
            &mut i,
        )
        .await?),
    }
}
