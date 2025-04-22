mod back;
mod jump;
mod play_pause;
mod restart;
mod seek;
mod skip;

pub use back::{Back, back};
pub use jump::{Autocomplete as JumpAutocomplete, Jump};
pub use play_pause::{PlayPause, play_pause};
pub use restart::Restart;
pub use seek::Seek;
pub use skip::{Skip, skip};

use crate::{
    LavalinkAndGuildIdAware,
    command::require,
    core::model::{BotStateAware, CacheAware, HttpAware},
    error::component::playback::HandleVoiceStateUpdateError,
    gateway::voice::Context,
};

#[tracing::instrument(skip_all, name = "voice_state_update")]
pub async fn handle_voice_state_update(
    ctx: &Context,
    connection_changed: bool,
) -> Result<(), HandleVoiceStateUpdateError> {
    let state = ctx.inner.as_ref();
    let maybe_old_state = ctx.old_voice_state();

    tracing::trace!("handling voice state update");
    let text_channel_id = {
        let Some(connection) = ctx.get_connection() else {
            tracing::trace!("no active connection");
            return Ok(());
        };

        if connection_changed {
            tracing::trace!("received connection change notification");
            return Ok(());
        }
        tracing::trace!("no connection change notification");

        connection.text_channel_id
    };

    let Ok(player) = require::player(ctx) else {
        return Ok(());
    };

    if state.user_id == ctx.bot().user_id()
        && state.suppress
        && maybe_old_state.is_some_and(|old_state| {
            state.channel_id.is_some_and(|channel_id| {
                channel_id == old_state.channel_id()
                    && ctx.cache().channel(channel_id).is_some_and(|channel| {
                        channel.kind == twilight_model::channel::ChannelType::GuildStageVoice
                    })
            }) && !old_state.suppress()
        })
    {
        player.set_pause(true).await?;
        ctx.http()
            .create_message(text_channel_id)
            .content("⚡▶ Paused `(Bot was moved to audience)`")
            .await?;
    }

    Ok(())
}
