use std::sync::Arc;

use twilight_lavalink::{model::IncomingEvent, Node};

use super::model::Process;
use crate::bot::{core::model::BotState, error::lavalink::ProcessResult};

pub async fn process(bot: Arc<BotState>, event: IncomingEvent, _node: Arc<Node>) -> ProcessResult {
    match event {
        IncomingEvent::TrackStart(ref e) => bot.as_track_start_context(e).process().await,
        IncomingEvent::TrackEnd(ref e) => bot.as_track_end_context(e).process().await,
        _ => Ok(()),
    }
}
