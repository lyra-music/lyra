use lavalink_rs::{
    client::LavalinkClient,
    hook,
    model::events::{TrackEnd, TrackException, TrackStart, TrackStuck},
};
use tokio::sync::RwLock;

use crate::bot::lavalink::{model::CorrectTrackInfo, PlayerData};

#[hook]
pub(super) async fn start(_: LavalinkClient, _session_id: String, event: &TrackStart) {
    tracing::debug!(
        "guild {} started {:?}",
        event.guild_id.0,
        event.track.info.checked_title()
    );
}

#[hook]
pub(super) async fn end(lavalink: LavalinkClient, _session_id: String, event: &TrackEnd) {
    let guild_id = event.guild_id;
    tracing::debug!(
        "guild {} ended   {:?}",
        guild_id.0,
        event.track.info.checked_title()
    );

    let ctx = lavalink
        .get_player_context(guild_id)
        .expect("player context must exist");

    let data = ctx
        .data::<RwLock<PlayerData>>()
        .expect("data type must be valid");
    let data_r = data.read().await;
    let queue = data_r.queue();

    // TODO: handle now playing message deleting
    if queue.advance_locked() {
        queue.advance_unlock();
    } else {
        let data = ctx
            .data::<RwLock<PlayerData>>()
            .expect("data type must be valid");
        let mut data_w = data.write().await;
        let queue = data_w.queue_mut();

        queue.advance();
        if let Some(item) = queue.current() {
            let Err(e) = ctx.play_now(item.track()).await else {
                return;
            };

            tracing::error!(?e);
        }
    }
}

#[hook]
pub(super) async fn exception(_: LavalinkClient, _session_id: String, _: &TrackException) {}

#[hook]
pub(super) async fn stuck(_: LavalinkClient, _session_id: String, _: &TrackStuck) {}
