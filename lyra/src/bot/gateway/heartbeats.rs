use super::Context;

pub async fn handle(ctx: Context) {
    ctx.bot()
        .update_latency(ctx.shard().read().await.latency().clone())
        .await;
}
