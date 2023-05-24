use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Context;
use tokio::sync::RwLock;
use twilight_gateway::{error::ReceiveMessageErrorType, CloseFrame, Event, MessageSender};
use twilight_gateway::{Intents, Shard, ShardId};

use super::gateway;
use super::lavalink;
use super::lib::models::Lyra;
use super::lib::models::LyraConfig;
use super::lib::traced;

pub struct BotManager {
    config: LyraConfig,
    shard: Arc<RwLock<Shard>>,
    sender: MessageSender,
    shutdown_flag: AtomicBool,
}

impl BotManager {
    pub fn new(config: LyraConfig) -> Self {
        let LyraConfig { token, .. } = &config;

        let intents: Intents =
            Intents::GUILD_MESSAGES | Intents::GUILD_VOICE_STATES | Intents::MESSAGE_CONTENT;
        let shard_id = ShardId::ONE;
        let shard = Shard::new(shard_id, token.clone(), intents);
        let sender = shard.sender();

        Self {
            config,
            shard: RwLock::new(shard).into(),
            sender,
            shutdown_flag: AtomicBool::new(false),
        }
    }

    fn is_shutting_down(&self) -> bool {
        self.shutdown_flag.load(Ordering::Relaxed)
    }

    pub async fn build_bot(&self) -> anyhow::Result<Lyra> {
        Ok(Lyra::new(self.config.clone(), self.shard.clone()).await?)
    }

    pub async fn handle_shutdown(&self, bot: Arc<Lyra>) -> anyhow::Result<()> {
        #[cfg(target_family = "unix")]
        {
            use tokio::signal::unix::{self, SignalKind};

            let mut sigint = unix::signal(SignalKind::interrupt())
                .context("unable to register SIGINT handler")?;
            let mut sigterm = unix::signal(SignalKind::terminate())
                .context("unable to register SIGTERM handler")?;

            tokio::select! {
                _ = sigint.recv() => tracing::debug!("received SIGINT"),
                _ = sigterm.recv() => tracing::debug!("received SIGTERM"),
            }
        }

        #[cfg(not(target_family = "unix"))]
        {
            use tokio::signal;

            signal::ctrl_c()
                .await
                .context("unable to register Ctrl+C handler")?;
        }

        tracing::info!("gracefully shutting down...");

        self.shutdown_flag.store(true, Ordering::Relaxed);
        tracing::debug!("set shutdown flag");

        self.sender.close(CloseFrame::NORMAL)?;
        tracing::debug!("sent gateway close");

        bot.disconnect_lavalink().await;
        tracing::debug!("sent lavalink disconnect");

        Ok(())
    }

    pub async fn handle_lavalink_events(&self, bot: Arc<Lyra>) -> anyhow::Result<()> {
        loop {
            match bot.next_lavalink_event().await {
                Some(event) => {
                    let ctx = lavalink::Context::new(event, bot.clone());

                    traced::tokio_spawn(lavalink::handle(ctx))
                }
                None if self.is_shutting_down() => {
                    tracing::debug!("lavalink shutdown");
                    return Ok(());
                }
                _ => {}
            }
        }
    }

    pub async fn handle_gateway_events(&self, bot: Arc<Lyra>) -> anyhow::Result<()> {
        loop {
            let event = match self.shard.write().await.next_event().await {
                Ok(Event::GatewayClose(_)) if self.is_shutting_down() => {
                    tracing::debug!("gateway closed");
                    break;
                }
                Ok(event) => event,
                Err(error)
                    if matches!(error.kind(), ReceiveMessageErrorType::Io)
                        && self.is_shutting_down() =>
                {
                    tracing::warn!("gateway closed via websocket connection error");
                    break;
                }
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

            let ctx = gateway::Context::new(event, bot.clone(), self.shard.clone());

            traced::tokio_spawn(gateway::handle(ctx))
        }
        Ok(())
    }
}
