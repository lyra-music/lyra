use lavalink_rs::{client::LavalinkClient, model::events::TrackEnd};

use crate::{
    command::require::cleanup_now_playing_message_and_play,
    error::lavalink::ProcessResult,
    lavalink::{CorrectTrackInfo, UnwrappedData},
};

#[tracing::instrument(err, skip_all, name = "track_end")]
pub(super) async fn impl_end(
    lavalink: LavalinkClient,
    _session_id: String,
    event: &TrackEnd,
) -> ProcessResult {
    let guild_id = event.guild_id;
    tracing::debug!(
        "guild {} ended   {:?}",
        guild_id.0,
        event.track.info.checked_title()
    );

    let Some(player) = lavalink.get_player_context(guild_id) else {
        tracing::debug!(?guild_id, "track ended via forced disconnection");

        return Ok(());
    };
    let data = player.data_unwrapped();

    let advancing_disabled = data.read().await.queue().advancing_disabled().await;
    if advancing_disabled {
        tracing::debug!(?guild_id, "track ended forcefully");
    } else {
        tracing::debug!(?guild_id, "track ended normally");
        let mut data_w = data.write().await;

        let cdata = &*lavalink.data_unwrapped();
        data_w.cleanup_now_playing_message(cdata).await;

        let queue = data_w.queue_mut();
        queue.advance();
        if let Some(index) = queue.current_index() {
            cleanup_now_playing_message_and_play(&player, cdata, index, &mut data_w).await?;
        }
        drop(data_w);
    }

    Ok(())
}
