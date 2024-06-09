use lavalink_rs::{
    client::LavalinkClient,
    hook,
    model::events::{TrackEnd, TrackException, TrackStart, TrackStuck},
};

use crate::{
    error::lavalink::ProcessResult,
    lavalink::{model::CorrectTrackInfo, UnwrappedPlayerData},
};

// FIXME: don't debug `LavalinkClient` until `lavalink_rs` stops stack overflowing

#[tracing::instrument(err, skip_all, name = "track_start")]
async fn impl_start(event: &TrackStart) -> ProcessResult {
    tracing::debug!(
        "guild {} started {:?}",
        event.guild_id.0,
        event.track.info.checked_title()
    );

    Ok(())
}

#[tracing::instrument(err, skip_all, name = "track_end")]
async fn impl_end(lavalink: LavalinkClient, event: &TrackEnd) -> ProcessResult {
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
    let queue = data_r.queue();

    // TODO: handle now playing message deleting
    if queue.advance_locked() {
        queue.advance_unlock();
    } else {
        drop(data_r);
        let mut data_w = data.write().await;
        let queue = data_w.queue_mut();

        queue.advance();
        if let Some(item) = queue.current() {
            player.play_now(item.track()).await?;
        }
    }

    Ok(())
}

#[tracing::instrument(err, skip_all, name = "track_exception")]
async fn impl_exception() -> ProcessResult {
    Ok(()) // TODO: handle track exception
}

#[tracing::instrument(err, skip_all, name = "track_stuck")]
async fn impl_stuck() -> ProcessResult {
    Ok(()) // TODO: handle track stuck
}

#[hook]
pub(super) async fn start(_: LavalinkClient, _session_id: String, event: &TrackStart) {
    let _ = impl_start(event).await;
}

#[hook]
pub(super) async fn end(lavalink: LavalinkClient, _session_id: String, event: &TrackEnd) {
    let _ = impl_end(lavalink, event).await;
}

#[hook]
pub(super) async fn exception(_: LavalinkClient, _session_id: String, _: &TrackException) {
    let _ = impl_exception().await;
}

#[hook]
pub(super) async fn stuck(_: LavalinkClient, _session_id: String, _: &TrackStuck) {
    let _ = impl_stuck().await;
}
