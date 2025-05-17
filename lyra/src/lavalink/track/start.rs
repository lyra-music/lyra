use lavalink_rs::{client::LavalinkClient, model::events::TrackStart};
use lyra_ext::num::u64_to_i64_truncating;

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
        u64_to_i64_truncating(guild_id.0)
    )
    .fetch_one(lavalink_data.db())
    .await?;

    if !rec.now_playing {
        return Ok(());
    }

    let msg_data =
        NowPlayingData::new_zeroed_timestamp(&lavalink_data, guild_id, &data_r, track).await?;
    drop(data_r);

    data.write()
        .await
        .new_now_playing_message(lavalink_data.http_owned(), msg_data)
        .await?;
    Ok(())
}
