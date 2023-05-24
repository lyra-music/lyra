use twilight_gateway::Event;
use twilight_model::application::interaction::InteractionType;

use super::{heartbeats, interactions, shards, Context};
use crate::bot::commands::models as command_models;

pub async fn handle(ctx: Context) -> anyhow::Result<()> {
    let bot = ctx.bot();
    match ctx.event {
        Event::Ready(_) => {
            shards::handle_ready(ctx)?;
        }
        Event::GatewayHeartbeatAck => {
            heartbeats::handle(ctx).await;
        }
        Event::InteractionCreate(i) => match i.kind {
            InteractionType::ApplicationCommand => {
                let ctx = command_models::Context::from_app_interaction(i, bot);
                interactions::handle_app(ctx).await?;
            }
            _ => todo!(),
        },
        _ => {}
    };

    Ok(())
}
