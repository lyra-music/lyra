#[lavalink_rs::hook]
pub(super) async fn ready(
    lavalink: lavalink_rs::client::LavalinkClient,
    _: String,
    _: &lavalink_rs::model::events::Ready,
) {
    if let Err(why) = lavalink.delete_all_player_contexts().await {
        tracing::error!("{why}:#?");
    }
}
