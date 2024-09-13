use lavalink_rs::{client::LavalinkClient, model::events::TrackEnd};

use crate::{
    error::lavalink::ProcessResult,
    lavalink::{CorrectTrackInfo, UnwrappedData},
};

#[allow(clippy::significant_drop_tightening)]
#[tracing::instrument(err, skip_all, name = "track_end")]
pub(super) async fn impl_end(
    lavalink: LavalinkClient,
    _: String,
    event: &TrackEnd,
) -> ProcessResult {
    let guild_id = event.guild_id;
    tracing::debug!(
        "guild {} ended   {:?}",
        guild_id.0,
        event.track.info.checked_title()
    );

    let Some(player) = lavalink.get_player_context(guild_id) else {
        tracing::trace!(?guild_id, "track ended via forced disconnection");

        return Ok(());
    };
    let data = player.data_unwrapped();
    let data_r = data.read().await;

    // TODO: handle now playing message deleting
    if data_r.queue().not_advance_locked().await {
        tracing::trace!(?guild_id, "track ended normally");

        drop(data_r);

        let mut data_w = data.write().await;
        let queue = data_w.queue_mut();

        queue.advance();
        if let Some(item) = queue.current() {
            player.play_now(item.data()).await?;
        }
    } else {
        tracing::trace!(?guild_id, "track ended forcefully");
    }

    Ok(())
}
