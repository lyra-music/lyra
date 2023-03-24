use std::sync::{Arc, RwLock};

use twilight_gateway::{Event, Shard};

use crate::bot::lib::models::LyraBot;

pub struct EventHandlerContext {
    pub event: Event,
    bot: Arc<LyraBot>,
    shard: Arc<RwLock<Shard>>,
}

impl EventHandlerContext {
    pub fn new(event: Event, bot: Arc<LyraBot>, shard: Arc<RwLock<Shard>>) -> Self {
        Self { event, bot, shard }
    }

    pub fn bot(&self) -> Arc<LyraBot> {
        self.bot.clone()
    }

    pub fn shard(&self) -> Arc<RwLock<Shard>> {
        self.shard.clone()
    }
}
