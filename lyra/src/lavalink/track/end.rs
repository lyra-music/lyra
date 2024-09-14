use lavalink_rs::{
    client::LavalinkClient,
    model::{events::TrackEnd, GuildId},
};
use lyra_ext::num::u64_to_i64_truncating;

use crate::{
    core::model::{DatabaseAware, HttpAware},
    error::lavalink::ProcessResult,
    lavalink::{model::PlayerData, CorrectTrackInfo, UnwrappedData},
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

    delete_now_playing_message(&*lavalink.data_unwrapped(), &data, guild_id).await?;

    let data_r = data.read().await;
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

pub async fn delete_now_playing_message(
    cx: &(impl HttpAware + DatabaseAware + Sync),
    data: &PlayerData,
    guild_id: GuildId,
) -> Result<(), sqlx::Error> {
    let rec = sqlx::query!(
        "SELECT now_playing FROM guild_configs WHERE id = $1;",
        u64_to_i64_truncating(guild_id.0)
    )
    .fetch_one(cx.db())
    .await?;

    if rec.now_playing {
        let mut data_w = data.write().await;
        if let Some(message_id) = data_w.take_now_playing_message_id() {
            let channel_id = data_w.now_playing_message_channel_id();
            let _ = cx.http().delete_message(channel_id, message_id).await;
            data_w.sync_now_playing_message_channel_id();
        };
    }

    Ok(())
}
