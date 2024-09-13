use lavalink_rs::{client::LavalinkClient, model::events::TrackException};

use crate::{core::model::HttpAware, error::lavalink::ProcessResult, lavalink::UnwrappedData};

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
            event.track.info.title
        ))
        .await?;

    Ok(())
}
