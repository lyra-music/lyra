use lavalink_rs::{
    client::LavalinkClient,
    model::events::{TrackEnd, TrackException, TrackStart, TrackStuck},
};
use lyra_proc::hook;
use tokio::sync::RwLock;

use crate::bot::lavalink::PlayerData;

#[hook]
pub(super) async fn start(_: LavalinkClient, _session_id: String, event: &TrackStart) {
    tracing::debug!(
        "guild {} started {}",
        event.guild_id.0,
        event.track.info.title
    );
}

#[hook]
pub(super) async fn end(client: LavalinkClient, _session_id: String, event: &TrackEnd) {
    let guild_id = event.guild_id;
    tracing::debug!("guild {} ended   {}", guild_id.0, event.track.info.title);

    let ctx = client
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
pub(super) async fn exception(client: LavalinkClient, session_id: String, event: &TrackException) {}

#[hook]
pub(super) async fn stuck(client: LavalinkClient, session_id: String, event: &TrackStuck) {}
