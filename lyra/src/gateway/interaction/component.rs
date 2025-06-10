use twilight_model::application::interaction::InteractionData;

use crate::{
    command::{model::ComponentCtx, require},
    core::r#static::component::NowPlayingButtonType,
    error::gateway::ProcessResult,
};

impl super::Context {
    pub(super) async fn process_as_component(mut self) -> ProcessResult {
        let Some(InteractionData::MessageComponent(data)) = self.inner.data.take() else {
            unreachable!()
        };
        tracing::trace!(?data);

        let ctx = ComponentCtx::from_data(self.inner, data, self.bot, self.latency, self.sender);
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
        let Some(current_track_title) = player_data_r
            .queue()
            .current()
            .map(|item| item.data().info.title.clone())
        else {
            return Ok(());
        };
        drop(player_data_r);

        let Some(now_playing_button) = ctx.take_custom_id_into_now_playing_button_type() else {
            return Ok(());
        };
        match now_playing_button {
            NowPlayingButtonType::Shuffle => {
                crate::component::queue::shuffle(player_data.clone(), &mut ctx, true).await?;
            }
            NowPlayingButtonType::Previous => {
                crate::component::playback::back(
                    Some(current_track_title),
                    player,
                    player_data.clone(),
                    &mut ctx,
                    true,
                )
                .await?;
            }
            NowPlayingButtonType::PlayPause => {
                crate::component::playback::play_pause(player, player_data.clone(), &mut ctx, true)
                    .await?;
            }
            NowPlayingButtonType::Next => {
                crate::component::playback::skip(
                    &current_track_title,
                    player,
                    player_data.clone(),
                    &mut ctx,
                    true,
                )
                .await?;
            }
            NowPlayingButtonType::Repeat => {
                let mode = crate::component::queue::get_next_repeat_mode(&ctx).await;
                crate::component::queue::repeat(&mut ctx, player_data.clone(), mode, true, true)
                    .await?;
            }
        }

        Ok(())
    }
}
