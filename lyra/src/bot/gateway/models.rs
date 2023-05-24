use std::sync::Arc;

use tokio::sync::RwLock;
use twilight_gateway::{Event, Shard};

use crate::bot::lib::models::Lyra;

pub struct Context {
    pub event: Event,
    bot: Arc<Lyra>,
    shard: Arc<RwLock<Shard>>,
}

impl Context {
    pub fn new(event: Event, bot: Arc<Lyra>, shard: Arc<RwLock<Shard>>) -> Self {
        Self { event, bot, shard }
    }

    pub fn bot(&self) -> Arc<Lyra> {
        self.bot.clone()
    }

    pub fn shard(&self) -> Arc<RwLock<Shard>> {
        self.shard.clone()
    }
}
