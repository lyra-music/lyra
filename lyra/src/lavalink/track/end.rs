use lavalink_rs::{client::LavalinkClient, model::events::TrackEnd};

use crate::{
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

    data.write()
        .await
        .delete_now_playing_message(lavalink.data_unwrapped().as_ref())
        .await;

    let data_r = data.read().await;
    if data_r.queue().advancing_disabled().await {
        tracing::debug!(?guild_id, "track ended forcefully");
    } else {
        tracing::debug!(?guild_id, "track ended normally");

        drop(data_r);
        let mut data_w = data.write().await;
        let queue = data_w.queue_mut();

        queue.advance();
        if let Some(item) = queue.current() {
            player.play_now(item.data()).await?;
        }
        drop(data_w);
    }

    Ok(())
}
