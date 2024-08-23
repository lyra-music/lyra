mod back;
mod jump;
mod play_pause;
mod restart;
mod seek;
mod skip;

pub use back::Back;
pub use jump::{Autocomplete as JumpAutocomplete, Jump};
pub use play_pause::PlayPause;
pub use restart::Restart;
pub use seek::Seek;
pub use skip::Skip;

use crate::{
    command::require,
    core::model::{BotStateAware, HttpAware},
    error::component::playback::HandleVoiceStateUpdateError,
    gateway::voice::Context,
    LavalinkAndGuildIdAware,
};

use super::connection::users_in_voice;

#[tracing::instrument(skip_all, name = "voice_state_update")]
pub async fn handle_voice_state_update(
    ctx: &Context,
    connection_changed: bool,
) -> Result<(), HandleVoiceStateUpdateError> {
    let state = ctx.inner.as_ref();
    let maybe_old_state = ctx.old_voice_state();

    tracing::trace!("handling voice state update");
    let (connected_channel_id, text_channel_id) = {
        let Some(connection) = ctx.get_connection() else {
            tracing::trace!("no active connection");
            return Ok(());
        };

        if connection_changed {
            tracing::trace!("received connection change notification");
            return Ok(());
        }
        tracing::trace!("no connection change notification");

        (connection.channel_id, connection.text_channel_id)
    };

    let Ok(player) = require::player(ctx) else {
        return Ok(());
    };

    if let Some(old_state) = maybe_old_state {
        let old_channel_id = old_state.channel_id();
        if state.user_id != ctx.bot().user_id()
            && old_channel_id == connected_channel_id
            && state.channel_id.is_some_and(|c| c != old_channel_id)
            && users_in_voice(ctx, connected_channel_id).is_some_and(|n| n == 0)
        {
            player.set_pause(true).await?;
            ctx.http()
                .create_message(text_channel_id)
                .content("⚡▶ Paused `(Bot is not used by anyone)`")
                .await?;
        }
    }

    Ok(())
}
