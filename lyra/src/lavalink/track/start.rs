use lavalink_rs::{client::LavalinkClient, model::events::TrackStart};

use crate::{
    core::model::{DatabaseAware, OwnedHttpAware},
    error::lavalink::ProcessResult,
    lavalink::{CorrectTrackInfo, UnwrappedData, model::NowPlayingData},
};

#[tracing::instrument(err, skip_all, name = "track_start")]
pub(super) async fn impl_start(
    lavalink: LavalinkClient,
    _session_id: String,
    event: &TrackStart,
) -> ProcessResult {
    let guild_id = event.guild_id;
    tracing::info!(
        "guild {} started {:?}",
        event.guild_id.0,
        event.track.info.checked_title()
    );

    let Some(player) = lavalink.get_player_context(guild_id) else {
        tracing::error!(?guild_id, "track started without player");

        return Ok(());
    };
    player
        .data_unwrapped()
        .write()
        .await
        .reset_track_timestamp();

    let data = player.data_unwrapped();
    let data_r = data.read().await;
    let queue = data_r.queue();
    let Some(track) = queue.current() else {
        return Ok(());
    };

    let lavalink_data = lavalink.data_unwrapped();
    let rec = sqlx::query!(
        "SELECT now_playing FROM guild_configs WHERE id = $1;",
        guild_id.0.cast_signed()
    )
    .fetch_one(lavalink_data.db())
    .await?;

    if !rec.now_playing {
        return Ok(());
    }

    let msg_data = NowPlayingData::new(&lavalink_data, guild_id, &data_r, track).await?;
    let now_playing_message_exists = data_r.now_playing_message_id().is_some();
    drop(data_r);

    let mut data_w = data.write().await;
    if now_playing_message_exists {
        data_w
            .update_and_apply_all_now_playing_data(msg_data)
            .await?;
    } else {
        data_w
            .new_now_playing_message(lavalink_data.http_owned(), msg_data)
            .await?;
    }
    drop(data_w);
    Ok(())
}
