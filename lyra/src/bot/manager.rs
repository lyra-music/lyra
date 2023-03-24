use std::sync::Arc;
use std::sync::RwLock;

use twilight_gateway::{Intents, Shard, ShardId};

use super::events;
use super::events::models::EventHandlerContext;
use super::lib::logging;
use super::lib::models::LyraBot;
use super::lib::models::LyraConfig;

pub struct LyraBotManager {
    config: LyraConfig,
    shard: Arc<RwLock<Shard>>,
}

impl LyraBotManager {
    pub fn new(config: LyraConfig) -> Self {
        let LyraConfig { token, .. } = &config;

        let intents: Intents =
            Intents::GUILD_MESSAGES | Intents::GUILD_VOICE_STATES | Intents::MESSAGE_CONTENT;
        let shard_id = ShardId::ONE;
        let shard = RwLock::new(Shard::new(shard_id, token.clone(), intents)).into();

        Self { config, shard }
    }

    pub async fn build_bot(&self) -> anyhow::Result<LyraBot> {
        Ok(LyraBot::new(&self.config, self.shard.clone()).await?)
    }

    pub async fn handle_events(&mut self, bot: Arc<LyraBot>) -> anyhow::Result<()> {
        loop {
            let event = match self
                .shard
                .write()
                .expect("`self.shard` must not be poisoned")
                .next_event()
                .await
            {
                Ok(event) => event,
                Err(source) => {
                    tracing::warn!(?source, "error receiving event");

                    if source.is_fatal() {
                        break;
                    }

                    continue;
                }
            };

            bot.cache().update(&event);
            bot.standby().process(&event);
            bot.lavalink().process(&event).await?;

            let ctx = EventHandlerContext::new(event, bot.clone(), self.shard.clone());

            logging::spawn(events::handle(ctx))
        }
        Ok(())
    }
}
