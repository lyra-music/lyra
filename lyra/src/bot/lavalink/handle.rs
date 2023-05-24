use twilight_lavalink::model::IncomingEvent;

use super::models::Context;

pub async fn handle(ctx: Context) -> anyhow::Result<()> {
    match ctx.event {
        IncomingEvent::Stats(_) => {}
        _ => todo!(),
    }

    Ok(())
}
