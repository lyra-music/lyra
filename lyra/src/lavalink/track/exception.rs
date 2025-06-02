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
    _session_id: String,
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
        let message = data.write().await.take_now_playing_message();
        if let Some(message) = message {
            let _ = lavalink
                .data_unwrapped()
                .http()
                .delete_message(message.channel_id(), message.id())
                .await;
        }
    }

    let oauth_enabled = std::env::var("PLUGINS_YOUTUBE_OAUTH_ENABLED")
        .is_ok_and(|x| x.parse::<bool>().is_ok_and(|y| y));
    let note = if oauth_enabled {
        "contact the bot developers to report the issue."
    } else {
        "contact the bot host to **enable YouTube OAuth**."
    };

    lavalink
        .data_unwrapped()
        .http()
        .create_message(data.read().await.text_channel_id())
        .content(&format!(
            "💔**`ー`** ~~`{}`~~ (Unable to play track)\n\
            -# Please ensure this track is available. \
            If you believe it should be playable, {}",
            event.track.info.title, note
        ))
        .await?;

    Ok(())
}
