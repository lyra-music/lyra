mod end;
mod exception;
mod start;
mod stuck;

use lavalink_rs::{
    client::LavalinkClient,
    hook,
    model::events::{TrackEnd, TrackException, TrackStart, TrackStuck},
};

#[hook]
pub(super) async fn start(lavalink: LavalinkClient, session_id: String, event: &TrackStart) {
    let _ = start::impl_start(lavalink, session_id, event).await;
}

#[hook]
pub(super) async fn end(lavalink: LavalinkClient, session_id: String, event: &TrackEnd) {
    let _ = end::impl_end(lavalink, session_id, event).await;
}

#[hook]
pub(super) async fn exception(
    lavalink: LavalinkClient,
    session_id: String,
    event: &TrackException,
) {
    let _ = exception::impl_exception(lavalink, session_id, event).await;
}

#[hook]
pub(super) async fn stuck(lavalink: LavalinkClient, session_id: String, event: &TrackStuck) {
    let _ = stuck::impl_stuck(lavalink, session_id, event).await;
}
