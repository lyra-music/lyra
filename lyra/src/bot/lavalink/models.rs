use std::sync::Arc;

use twilight_lavalink::model::IncomingEvent;

use crate::bot::lib::models::Lyra;

pub struct Context {
    pub event: IncomingEvent,
    bot: Arc<Lyra>,
}

impl Context {
    pub fn new(event: IncomingEvent, bot: Arc<Lyra>) -> Self {
        Self { event, bot }
    }

    pub fn bot(&self) -> Arc<Lyra> {
        self.bot.clone()
    }
}
