use lavalink_rs::{client::LavalinkClient, model::events::TrackException};

use crate::{core::model::HttpAware, error::lavalink::ProcessResult, lavalink::UnwrappedData};

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

    let data = player.data_unwrapped();
    data.write()
        .await
        .cleanup_now_playing_message(&*lavalink.data_unwrapped())
        .await;

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
