use std::{fs, iter, sync::Arc, thread};

use anyhow::{Context, Result};
use futures::future;
use tokio::{
    sync::{watch, RwLock},
    task::JoinSet,
};
use tokio_stream::StreamExt;
use twilight_gateway::{
    stream::{self, ShardEventStream, ShardRef},
    CloseFrame, Config as ShardConfig, Event,
};
use twilight_gateway::{ConfigBuilder, Intents, Shard};
use twilight_http::{client::ClientBuilder, Client};
use twilight_lavalink::model::IncomingEvent;
use twilight_model::{
    channel::message::AllowedMentions,
    gateway::{
        payload::outgoing::update_presence::UpdatePresencePayload,
        presence::{Activity, ActivityType, MinimalActivity, Status},
    },
    id::{marker::UserMarker, Id},
};

use crate::bot::{
    lavalink::LavalinkManager,
    lib::consts::metadata::{
        AUTHORS, COPYRIGHT, OS_INFO, REPOSITORY, RUST_VERSION, SUPPORT, VERSION,
    },
};

use super::lib::{
    models::{Cacheful, Config, Lyra},
    traced,
};
use super::{
    gateway::{self, OldResources},
    lavalink::{self, Lavalink},
};

pub(super) struct BotManager {
    config: Config,
    shard_config: ShardConfig,
    http: Arc<Client>,
}

impl BotManager {
    const INTENTS: Intents = Intents::GUILDS.union(Intents::GUILD_VOICE_STATES);

    pub(super) fn new(config: Config) -> Self {
        let Config { ref token, .. } = config;

        let http = ClientBuilder::default()
            .default_allowed_mentions(AllowedMentions::default())
            .token(token.to_owned())
            .build()
            .into();

        let shard_config = ConfigBuilder::new(token.clone(), Self::INTENTS)
            .presence(
                UpdatePresencePayload::new(
                    [Activity::from(MinimalActivity {
                        kind: ActivityType::Listening,
                        name: "/play".into(),
                        url: None,
                    })],
                    false,
                    None,
                    Status::Online,
                )
                .expect("activities must not be empty"),
            )
            .build();

        Self {
            config,
            http,
            shard_config,
        }
    }

    pub(super) async fn start(&self) -> Result<()> {
        let shards = self.build_and_split_shards().await?;

        let (tx, rx) = watch::channel(false);

        let mut set = JoinSet::new();

        let bot = Arc::new(Lyra::new(self.config.clone(), self.http.clone()).await?);
        bot.register_app_commands().await?;

        let user_id = self.http.current_user().await?.model().await?.id;

        let shard_count = shards.iter().map(|s| s.len() as u64).sum();
        let lavalink_and_manager = self.build_lavalink_manager(shard_count, user_id).await?;

        for mut shards in shards {
            let mut rx = rx.clone();
            let bot = bot.clone();
            let (lavalink, lavalink_manager) = lavalink_and_manager.clone();

            set.spawn(async move {
                tokio::select! {
                    _ = Self::handle_gateway_events(shards.iter_mut(), lavalink.clone(), bot.clone()) => {},
                    _ = Self::handle_lavalink_events(lavalink_manager.clone(), lavalink.clone(), bot) => {},
                    _ = rx.changed() => {
                        future::join_all(shards.iter_mut().map(|shard| async move {
                            shard.close(CloseFrame::NORMAL).await
                        })).await;
                    }
                }
            });
        }

        Self::print_banner();
        Self::wait_for_shutdown().await?;

        tracing::info!("gracefully shutting down...");

        tx.send(true)?;

        while set.join_next().await.is_some() {}

        Ok(())
    }

    async fn build_and_split_shards(&self) -> Result<Vec<Vec<Shard>>> {
        let tasks = thread::available_parallelism()?.get();

        let init = iter::repeat_with(Vec::new)
            .take(tasks)
            .collect::<Vec<Vec<_>>>();
        let shards =
            stream::create_recommended(&self.http, self.shard_config.clone(), |_, builder| {
                builder.build()
            })
            .await?
            .enumerate()
            .fold(init, |mut fold, (idx, shard)| {
                fold[idx % tasks].push(shard);
                fold
            });

        Ok(shards)
    }

    async fn build_lavalink_manager(
        &self,
        shard_count: u64,
        user_id: Id<UserMarker>,
    ) -> Result<(Arc<Lavalink>, Arc<RwLock<LavalinkManager>>)> {
        let Config {
            ref lavalink_addr,
            ref lavalink_auth,
            ..
        } = self.config;
        let client = twilight_lavalink::Lavalink::new(user_id, shard_count);

        let node_1 = client.add(*lavalink_addr, lavalink_auth).await?;
        let node_2 = client.add(*lavalink_addr, lavalink_auth).await?;

        let lavalink = Arc::new(Lavalink::new(client));
        let manager = Arc::new(RwLock::new(LavalinkManager::new([node_1, node_2].into())));

        Ok((lavalink, manager))
    }

    async fn handle_lavalink_events(
        lavalink_manager: Arc<RwLock<LavalinkManager>>,
        lavalink: Arc<Lavalink>,
        bot: Arc<Lyra>,
    ) -> Result<()> {
        loop {
            if let Some((_, event)) = lavalink_manager
                .write()
                .await
                .incoming_events()
                .next()
                .await
            {
                Self::process_lavalink_events(event, lavalink.clone(), bot.clone()).await?
            }
        }
    }

    async fn process_lavalink_events(
        event: IncomingEvent,
        lavalink: Arc<Lavalink>,
        bot: Arc<Lyra>,
    ) -> Result<()> {
        let bot = lavalink::ContextedLyra::new(event, bot, lavalink);
        traced::tokio_spawn(bot.process());

        Ok(())
    }

    async fn handle_gateway_events(
        shards: impl Iterator<Item = &mut Shard>,
        lavalink: Arc<Lavalink>,
        bot: Arc<Lyra>,
    ) -> Result<()> {
        let mut stream = ShardEventStream::new(shards);
        loop {
            let (shard, event) = match stream.next().await {
                Some((shard, Ok(event))) => (shard, event),
                Some((_, Err(source))) => {
                    tracing::warn!(?source, "error receiving event");

                    if source.is_fatal() {
                        break Ok(());
                    }

                    continue;
                }
                None => break Ok(()),
            };

            Self::process_gateway_events(shard, event, lavalink.clone(), bot.clone()).await?
        }
    }

    async fn process_gateway_events(
        shard: ShardRef<'_>,
        event: Event,
        lavalink: Arc<Lavalink>,
        bot: Arc<Lyra>,
    ) -> Result<()> {
        let old_resources = OldResources::new(bot.cache(), &event);

        bot.cache().update(&event);
        bot.standby().process(&event);
        lavalink.process(&event).await?;

        let bot = gateway::ContextedLyra::new(event, old_resources, bot, shard, lavalink);
        traced::tokio_spawn(bot.process());

        Ok(())
    }

    async fn wait_for_shutdown() -> Result<()> {
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

        Ok(())
    }

    fn print_banner() {
        let path = "../assets/lyra2-ascii.ans";
        let banner = fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("`{path}` must exist"))
            .replace("%version", VERSION)
            .replace("%copyright", COPYRIGHT)
            .replace("%authors", AUTHORS)
            .replace("%repository", REPOSITORY)
            .replace("%support", SUPPORT)
            .replace("%rust", &RUST_VERSION)
            .replace("%os_info", &OS_INFO);
        println!("{banner}");
    }
}
