use lavalink_rs::{client::LavalinkClient, model::events::TrackException};
use lyra_ext::num::u64_to_i64_truncating;

use crate::{
    core::model::{DatabaseAware, HttpAware},
    error::lavalink::ProcessResult,
    lavalink::UnwrappedData,
};

#[tracing::instrument(err, skip_all, name = "track_exception")]
pub(super) async fn impl_exception(
    lavalink: LavalinkClient,
    _: String,
    event: &TrackException,
) -> ProcessResult {
    let guild_id = event.guild_id;
    tracing::error!(?event, "track exception");

    let Some(player) = lavalink.get_player_context(guild_id) else {
        return Ok(());
    };

    let rec = sqlx::query!(
        "SELECT now_playing FROM guild_configs WHERE id = $1;",
        u64_to_i64_truncating(guild_id.0)
    )
    .fetch_one(lavalink.data_unwrapped().db())
    .await?;

    let data = player.data_unwrapped();
    if rec.now_playing {
        let mut data_w = data.write().await;
        if let Some(message_id) = data_w.take_now_playing_message_id() {
            let channel_id = data_w.now_playing_message_channel_id();
            let _ = lavalink
                .data_unwrapped()
                .http()
                .delete_message(channel_id, message_id)
                .await;
            data_w.sync_now_playing_message_channel_id();
        };
    }

    lavalink
        .data_unwrapped()
        .http()
        .create_message(data.read().await.text_channel_id())
        .content(&format!(
            "💔**`ー`** ~~`{}`~~ `(Error playing this track)`",
            event.track.info.title
        ))
        .await?;

    Ok(())
}
