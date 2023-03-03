use std::sync::Arc;
use twilight_gateway::{Event, Intents, Shard, ShardId};

use super::lib::models::LyraConfig;
use super::modules::connections::{join, leave};
use super::modules::playback::{pause, seek, stop};
use super::modules::queue::play;
use super::modules::tuning::volume;
use super::{
    events::message_creates,
    lib::models::{Context, LyraBot},
};
use crate::bot::lib::logging::spawn;

pub struct LyraBotManager {
    config: LyraConfig,
    pub shard: Shard,
}

impl LyraBotManager {
    pub fn new(config: LyraConfig) -> Self {
        let LyraConfig { token, .. } = &config;

        let intents: Intents =
            Intents::GUILD_MESSAGES | Intents::GUILD_VOICE_STATES | Intents::MESSAGE_CONTENT;
        let shard_id = ShardId::ONE;
        let shard = Shard::new(shard_id, token.clone(), intents);

        Self { config, shard }
    }

    pub async fn build_bot(&self) -> anyhow::Result<LyraBot> {
        Ok(LyraBot::new(&self.config, &self.shard).await?)
    }

    pub async fn handle_events(&mut self, bot: Arc<LyraBot>) -> anyhow::Result<()> {
        loop {
            let event = match self.shard.next_event().await {
                Ok(event) => event,
                Err(source) => {
                    tracing::warn!(?source, "error receiving event");

                    if source.is_fatal() {
                        break;
                    }

                    continue;
                }
            };

            bot.standby.process(&event);
            bot.lavalink.process(&event).await?;
            bot.cache.update(&event);

            match event {
                Event::MessageCreate(msg) => message_creates::handle!(msg, bot),
                _ => {}
            }
        }

        Ok(())
    }
}
