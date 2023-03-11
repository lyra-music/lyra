use std::sync::Arc;

use twilight_gateway::Event;
use twilight_model::application::interaction::InteractionType;

use super::interactions;
use crate::bot::{commands::models::Context, lib::models::LyraBot};

pub async fn handle(event: Event, bot: Arc<LyraBot>) -> anyhow::Result<()> {
    match event {
        Event::InteractionCreate(i) => match i.kind {
            InteractionType::ApplicationCommand => {
                let ctx = Context::from_app_interaction(i, bot.clone());
                interactions::handle(ctx).await?;
            }
            _ => {}
        },
        _ => {}
    };

    Ok(())
}
