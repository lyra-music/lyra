use std::sync::Arc;
use std::sync::Mutex;

use twilight_gateway::{Intents, Shard, ShardId};

use super::events;
use super::lib::logging;
use super::lib::models::LyraBot;
use super::lib::models::LyraConfig;

pub struct LyraBotManager {
    config: LyraConfig,
    shard: Arc<Mutex<Shard>>,
}

impl LyraBotManager {
    pub fn new(config: LyraConfig) -> Self {
        let LyraConfig { token, .. } = &config;

        let intents: Intents =
            Intents::GUILD_MESSAGES | Intents::GUILD_VOICE_STATES | Intents::MESSAGE_CONTENT;
        let shard_id = ShardId::ONE;
        let shard = Mutex::new(Shard::new(shard_id, token.clone(), intents)).into();

        Self { config, shard }
    }

    pub async fn build_bot(&self) -> anyhow::Result<LyraBot> {
        Ok(LyraBot::new(&self.config, self.shard.clone()).await?)
    }

    pub async fn handle_events(&self, bot: Arc<LyraBot>) -> anyhow::Result<()> {
        loop {
            let event = match self
                .shard
                .lock()
                .expect("another user of `self.shard` must not panick while holding it")
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

            logging::spawn(events::handle(event, bot.clone()))
        }

        Ok(())
    }
}
