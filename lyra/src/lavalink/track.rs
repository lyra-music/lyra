use lavalink_rs::{
    client::LavalinkClient,
    hook,
    model::events::{TrackEnd, TrackException, TrackStart, TrackStuck},
};

use crate::{
    core::model::HttpAware,
    error::lavalink::ProcessResult,
    lavalink::{model::CorrectTrackInfo, UnwrappedData},
};

#[tracing::instrument(err, skip_all, name = "track_start")]
async fn impl_start(lavalink: LavalinkClient, _: String, event: &TrackStart) -> ProcessResult {
    let guild_id = event.guild_id;
    tracing::debug!(
        "guild {} started {:?}",
        event.guild_id.0,
        event.track.info.checked_title()
    );

    let Some(player) = lavalink.get_player_context(guild_id) else {
        tracing::warn!(?guild_id, "track started without player");

        return Ok(());
    };
    player
        .data_unwrapped()
        .write()
        .await
        .reset_track_timestamp();

    Ok(())
}

#[allow(clippy::significant_drop_tightening)]
#[tracing::instrument(err, skip_all, name = "track_end")]
async fn impl_end(lavalink: LavalinkClient, _: String, event: &TrackEnd) -> ProcessResult {
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

#[tracing::instrument(err, skip_all, name = "track_exception")]
async fn impl_exception(
    lavalink: LavalinkClient,
    _: String,
    event: &TrackException,
) -> ProcessResult {
    let guild_id = event.guild_id;
    tracing::error!(?event, "track exception");

    let Some(player) = lavalink.get_player_context(guild_id) else {
        return Ok(());
    };

    let channel_id = {
        let data = player.data_unwrapped();
        let data_r = data.read().await;
        data_r.text_channel_id()
    };

    lavalink
        .data_unwrapped()
        .http()
        .create_message(channel_id)
        .content(&format!(
            "💔**`ー`** ~~`{}`~~ `(Error playing this track)`",
            event.track.info.title()
        ))
        .await?;

    Ok(())
}

#[tracing::instrument(err, skip_all, name = "track_stuck")]
async fn impl_stuck(lavalink: LavalinkClient, _: String, event: &TrackStuck) -> ProcessResult {
    let guild_id = event.guild_id;
    tracing::warn!(?event, "track stuck");

    let Some(player) = lavalink.get_player_context(guild_id) else {
        return Ok(());
    };

    let channel_id = player.data_unwrapped().read().await.text_channel_id();
    lavalink
        .data_unwrapped()
        .http()
        .create_message(channel_id)
        .content("🌀 Playback interrupted. Please wait or try using the bot again later.")
        .await?;

    Ok(())
}

#[hook]
pub(super) async fn start(lavalink: LavalinkClient, session_id: String, event: &TrackStart) {
    let _ = impl_start(lavalink, session_id, event).await;
}

#[hook]
pub(super) async fn end(lavalink: LavalinkClient, session_id: String, event: &TrackEnd) {
    let _ = impl_end(lavalink, session_id, event).await;
}

#[hook]
pub(super) async fn exception(
    lavalink: LavalinkClient,
    session_id: String,
    event: &TrackException,
) {
    let _ = impl_exception(lavalink, session_id, event).await;
}

#[hook]
pub(super) async fn stuck(lavalink: LavalinkClient, session_id: String, event: &TrackStuck) {
    let _ = impl_stuck(lavalink, session_id, event).await;
}
