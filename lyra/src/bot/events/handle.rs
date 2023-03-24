use twilight_gateway::Event;
use twilight_model::application::interaction::InteractionType;

use super::{heartbeats, interactions, models::EventHandlerContext};
use crate::bot::commands::models::Context;

pub async fn handle(ctx: EventHandlerContext) -> anyhow::Result<()> {
    let bot = ctx.bot();
    match ctx.event {
        Event::InteractionCreate(i) => match i.kind {
            InteractionType::ApplicationCommand => {
                let ctx = Context::from_app_interaction(i, bot);
                interactions::handle_app(ctx).await?;
            }
            _ => todo!(),
        },
        Event::GatewayHeartbeatAck => {
            heartbeats::handle(ctx);
        }
        _ => {}
    };

    Ok(())
}
