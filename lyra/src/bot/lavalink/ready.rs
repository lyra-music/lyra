#[tracing::instrument(err, skip_all)]
async fn impl_ready(
    lavalink: lavalink_rs::client::LavalinkClient,
) -> crate::bot::error::lavalink::ProcessResult {
    lavalink.delete_all_player_contexts().await?;

    Ok(())
}

#[lavalink_rs::hook]
pub(super) async fn ready(
    lavalink: lavalink_rs::client::LavalinkClient,
    _: String,
    _: &lavalink_rs::model::events::Ready,
) {
    let _ = impl_ready(lavalink).await;
}
