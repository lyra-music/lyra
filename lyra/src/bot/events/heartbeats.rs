use super::models::EventHandlerContext;

pub fn handle(ctx: EventHandlerContext) {
    ctx.bot().update_latency(
        ctx.shard()
            .read()
            .expect("`ctx.shard()` must not be poisoned")
            .latency()
            .clone(),
    );
}
