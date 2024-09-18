use lavalink_rs::{client::LavalinkClient, model::events::TrackStuck};

use crate::{core::model::HttpAware, error::lavalink::ProcessResult, lavalink::UnwrappedData};

#[tracing::instrument(err, skip_all, name = "track_stuck")]
pub(super) async fn impl_stuck(
    lavalink: LavalinkClient,
    _: String,
    event: &TrackStuck,
) -> ProcessResult {
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
