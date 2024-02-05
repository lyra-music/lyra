use std::{fs, mem, str::FromStr, sync::Arc};

use log::LevelFilter;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions,
};
use tokio::{sync::watch, task::JoinSet};
use tokio_stream::StreamExt;
use twilight_gateway::{
    stream::{self, StartRecommendedError},
    CloseFrame, Config as ShardConfig, Event,
};
use twilight_gateway::{ConfigBuilder, Intents, Shard};
use twilight_http::{client::ClientBuilder, Client};
use twilight_lavalink::{
    client::ClientError,
    model::IncomingEvent,
    node::{IncomingEvents, NodeError},
    Node,
};
use twilight_model::{
    channel::message::AllowedMentions,
    gateway::{
        payload::outgoing::update_presence::UpdatePresencePayload,
        presence::{Activity, ActivityType, MinimalActivity, Status},
    },
    id::{marker::UserMarker, Id},
};

use crate::bot::core::r#const::metadata::{
    AUTHORS, COPYRIGHT, OS_INFO, REPOSITORY, RUST_VERSION, SUPPORT, VERSION,
};

use super::{
    core::{
        model::{BotState, CacheAware, Config},
        traced,
    },
    error::manager::{StartError, WaitForShutdownError},
    gateway,
    lavalink::{self, NodeAndReceiver},
};
use super::{
    gateway::LastCachedStates,
    lavalink::{ClientAware, Lavalink},
};

pub(super) struct BotManager {
    config: Config,
    shard_config: Option<ShardConfig>,
    http: Option<Client>,
}

impl BotManager {
    const INTENTS: Intents = Intents::GUILDS.union(Intents::GUILD_VOICE_STATES);
    const LAVALINK_NODE_COUNT: usize = 1;

    pub(super) fn new(mut config: Config) -> Self {
        let token = mem::take(&mut config.token);

        let http = ClientBuilder::default()
            .default_allowed_mentions(AllowedMentions::default())
            .token(token.clone())
            .build()
            .into();

        let shard_config = Some(
            ConfigBuilder::new(token, Self::INTENTS)
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
                .build(),
        );

        Self {
            config,
            shard_config,
            http,
        }
    }

    pub(super) async fn start(&mut self) -> Result<(), StartError> {
        let http = self.http.take().expect("`BotManager::http` must exist");
        let shards = self.build_and_split_shards(&http).await?;

        let (tx, rx) = watch::channel(false);

        let mut set = JoinSet::new();

        let database_url = mem::take(&mut self.config.database_url);
        let options = PgConnectOptions::from_str(&database_url)?.log_statements(LevelFilter::Debug);
        let db = PgPoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let shard_count = shards.len() as u64;
        let user_id = http.current_user().await?.model().await?.id;

        let (lavalink, nodes_and_receivers) =
            self.build_lavalink_nodes(shard_count, user_id).await?;

        let bot = Arc::new(BotState::new(db, http, lavalink));
        bot.interaction().await?.register_global_commands().await?;

        for mut shard in shards {
            let mut rx = rx.clone();
            let bot = bot.clone();

            set.spawn(async move {
                tokio::select! {
                    _ = Self::handle_gateway_events(&mut shard, bot.clone()) => {},
                    _ = rx.changed() => {
                        _ = shard.close(CloseFrame::NORMAL).await;
                    }
                }
            });
        }

        for (node, mut node_rx) in nodes_and_receivers {
            let mut rx = rx.clone();
            let bot = bot.clone();
            let addr = node.config().address;

            set.spawn(async move {
                tokio::select! {
                    () = Self::handle_lavalink_events(&mut node_rx, node, bot.clone()) => {},
                    _ = rx.changed() => {
                        _ = bot.lavalink().disconnect(addr);
                    }
                }
            });
        }

        Self::print_banner();
        Self::wait_for_shutdown().await?;

        tracing::info!("gracefully shutting down...");

        tx.send(true)?;

        while set.join_next().await.is_some() {}

        tracing::info!("shut down gracefully");
        Ok(())
    }

    async fn build_and_split_shards(
        &mut self,
        client: &Client,
    ) -> Result<Vec<Shard>, StartRecommendedError> {
        let shard_config = self
            .shard_config
            .take()
            .expect("`BotManager::shard_config` must exist");

        let shards = stream::create_recommended(client, shard_config, |_, builder| builder.build())
            .await?
            .collect();
        Ok(shards)
    }

    async fn build_lavalink_nodes(
        &self,
        shard_count: u64,
        user_id: Id<UserMarker>,
    ) -> Result<(Lavalink, [NodeAndReceiver; Self::LAVALINK_NODE_COUNT]), NodeError> {
        let Config {
            ref lavalink_addr,
            ref lavalink_auth,
            ..
        } = self.config;

        let client = twilight_lavalink::Lavalink::new(user_id, shard_count);

        let node_1 = client.add(*lavalink_addr, lavalink_auth).await?;

        Ok((client.into(), [node_1]))
    }

    async fn handle_lavalink_events(
        incoming_events: &mut IncomingEvents,
        node: Arc<Node>,
        bot: Arc<BotState>,
    ) {
        loop {
            if let Some(event) = incoming_events.next().await {
                Self::process_lavalink_events(node.clone(), event, bot.clone());
            }
        }
    }

    fn process_lavalink_events(node: Arc<Node>, event: IncomingEvent, bot: Arc<BotState>) {
        traced::tokio_spawn(lavalink::process(bot, event, node));
    }

    async fn handle_gateway_events(
        shard: &mut Shard,
        bot: Arc<BotState>,
    ) -> Result<(), ClientError> {
        loop {
            let event = match shard.next_event().await {
                Ok(event) => event,
                Err(source) => {
                    tracing::warn!(?source, "error receiving event");

                    if source.is_fatal() {
                        break Ok(());
                    }

                    continue;
                }
            };

            Self::process_gateway_events(shard, event, bot.clone()).await?;
        }
    }

    async fn process_gateway_events(
        shard: &Shard,
        event: Event,
        bot: Arc<BotState>,
    ) -> Result<(), ClientError> {
        let states = LastCachedStates::new(bot.cache(), &event);

        bot.cache().update(&event);
        bot.standby().process(&event);
        bot.lavalink().process(&event).await?;

        traced::tokio_spawn(gateway::process(
            bot,
            event,
            states,
            shard.latency().clone(),
            shard.sender(),
        ));

        Ok(())
    }

    async fn wait_for_shutdown() -> Result<(), WaitForShutdownError> {
        #[cfg(target_family = "unix")]
        {
            use tokio::signal::unix::{self, SignalKind};

            let mut sigint = unix::signal(SignalKind::interrupt())?;
            let mut sigterm = unix::signal(SignalKind::terminate())?;

            tokio::select! {
                _ = sigint.recv() => tracing::debug!("received SIGINT"),
                _ = sigterm.recv() => tracing::debug!("received SIGTERM"),
            }
        }

        #[cfg(not(target_family = "unix"))]
        {
            use tokio::signal;

            signal::ctrl_c().await?;
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
