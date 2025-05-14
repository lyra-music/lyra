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
    command::require,
    core::model::{BotStateAware, CacheAware, HttpAware},
    error::component::playback::HandleVoiceStateUpdateError,
    gateway::voice::Context,
    lavalink::ConnectionHead,
};

#[tracing::instrument(skip_all, name = "playback")]
pub async fn handle_voice_state_update(
    ctx: &Context,
    head: ConnectionHead,
) -> Result<(), HandleVoiceStateUpdateError> {
    let state = ctx.inner.as_ref();
    let maybe_old_state = ctx.old_voice_state();

    tracing::debug!("handling voice state update");
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
            .create_message(head.text_channel_id())
            .content("⚡▶ Paused `(Bot was moved to audience).`")
            .await?;
    }

    Ok(())
}
