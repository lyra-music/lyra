use std::{
    env,
    str::FromStr,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, Ordering},
    },
};

use lavalink_rs::{client::LavalinkClient, model::client::NodeDistributionStrategy};
use log::LevelFilter;
use sqlx::{
    ConnectOptions, migrate,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::task::JoinHandle;
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::{
    CloseFrame, Config as ShardConfig, ConfigBuilder, Event, EventTypeFlags, Intents,
    MessageSender, Shard, StreamExt, error::StartRecommendedError,
};
use twilight_http::{Client, client::ClientBuilder};
use twilight_model::{
    channel::message::AllowedMentions,
    gateway::{
        payload::outgoing::update_presence::UpdatePresencePayload,
        presence::{Activity, ActivityType, MinimalActivity, Status},
    },
    id::{Id, marker::UserMarker},
};

use crate::{
    LavalinkAware,
    core::r#const::metadata::BANNER,
    lavalink::{ClientData, handlers},
};

use super::{
    core::{
        model::{BotState, CacheAware, Config},
        traced,
    },
    error::runner::{StartError, WaitForSignalError, WaitUntilShutdownError},
    gateway,
    lavalink::DelegateMethods,
};
use super::{gateway::LastCachedStates, lavalink::Lavalink};

static CONFIG: LazyLock<Config> = LazyLock::new(|| Config {
    token: env::var("BOT_TOKEN").expect("missing BOT_TOKEN").leak(),
    lavalink_host: format!(
        "{}:{}",
        env::var("SERVER_ADDRESS").expect("missing SERVER_ADDRESS"),
        env::var("SERVER_PORT").expect("missing SERVER_PORT")
    )
    .leak(),
    lavalink_pwd: env::var("LAVALINK_SERVER_PASSWORD")
        .expect("missing LAVALINK_SERVER_PASSWORD")
        .leak(),
    database_url: env::var("DATABASE_URL")
        .expect("missing DATABASE_URL")
        .leak(),
});

const INTENTS: Intents = Intents::GUILDS
    .union(Intents::GUILD_VOICE_STATES)
    .union(Intents::GUILD_MESSAGES);

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

fn build_http_client() -> Arc<Client> {
    ClientBuilder::default()
        .default_allowed_mentions(AllowedMentions::default())
        .token(CONFIG.token.to_owned())
        .build()
        .into()
}

fn build_shard_config() -> ShardConfig {
    ConfigBuilder::new(CONFIG.token.to_owned(), INTENTS)
        .presence(
            UpdatePresencePayload::new(
                [Activity::from(MinimalActivity {
                    kind: ActivityType::Listening,
                    name: String::from("/play"),
                    url: None,
                })],
                false,
                None,
                Status::Online,
            )
            .expect("provided set of activities must be non-empty"),
        )
        .build()
}

pub async fn start() -> Result<(), StartError> {
    tracing::debug!("began starting the bot");

    let db = PgPoolOptions::new()
        .max_connections(20)
        .connect_with(
            PgConnectOptions::from_str(CONFIG.database_url)?.log_statements(LevelFilter::Debug),
        )
        .await?;

    migrate!("../migrations").run(&db).await?;

    let http = build_http_client();

    let cache = Arc::new(InMemoryCache::new());
    let data = ClientData::new(http.clone(), cache.clone(), db.clone());
    let user_id = http.current_user().await?.model().await?.id;
    let lavalink = build_lavalink_client(user_id, data).await;

    let shards = build_and_split_shards(&http).await?;
    let shards_len = shards.len();
    let mut senders = Vec::with_capacity(shards_len);
    let mut tasks = Vec::with_capacity(shards_len);
    let bot = Arc::new(BotState::new(db, http, cache, lavalink));

    for shard in shards {
        senders.push(shard.sender());
        tasks.push(tokio::spawn(handle_gateway_events(shard, bot.clone())));
    }

    println!("{}", *BANNER);
    Ok(wait_until_shutdown(senders, tasks, &bot).await?)
}

async fn build_and_split_shards(
    client: &Client,
) -> Result<impl ExactSizeIterator<Item = Shard> + use<>, StartRecommendedError> {
    let shard_config = build_shard_config();
    let shards =
        twilight_gateway::create_recommended(client, shard_config, |_, builder| builder.build())
            .await?;
    Ok(shards)
}

#[tracing::instrument(skip_all, name = "lavalink")]
async fn build_lavalink_client(user_id: Id<UserMarker>, data: ClientData) -> Lavalink {
    let events = handlers();

    let nodes = Vec::from([lavalink_rs::node::NodeBuilder {
        hostname: CONFIG.lavalink_host.to_owned(),
        password: CONFIG.lavalink_pwd.to_owned(),
        user_id: user_id.into(),
        ..Default::default()
    }]);

    let strategy = NodeDistributionStrategy::new();
    let client = LavalinkClient::new_with_data(events, nodes, strategy, data.into()).await;
    client.into()
}

#[tracing::instrument(skip_all, name = "gateway")]
async fn handle_gateway_events(mut shard: Shard, bot: Arc<BotState>) {
    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let event = match item {
            Ok(Event::GatewayClose(_)) if SHUTDOWN.load(Ordering::Relaxed) => break,
            Ok(event) => event,
            Err(source) => {
                tracing::warn!(?source, "error receiving event");

                continue;
            }
        };

        tracing::trace!(?event, shard = ?shard.id(), "received event");
        process_gateway_events(&shard, event, bot.clone());
    }
}

fn process_gateway_events(shard: &Shard, event: Event, bot: Arc<BotState>) {
    let states = LastCachedStates::new(bot.cache(), &event);

    bot.cache().update(&event);
    bot.standby().process(&event);
    bot.lavalink().process(&event);

    traced::tokio_spawn(gateway::process(
        bot,
        event,
        states,
        shard.id(),
        shard.latency().clone(),
        shard.sender(),
    ));
}

#[tracing::instrument]
async fn wait_for_signal() -> Result<(), WaitForSignalError> {
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

#[tracing::instrument(skip_all, name = "shutdown")]
async fn wait_until_shutdown(
    senders: Vec<MessageSender>,
    tasks: Vec<JoinHandle<()>>,
    bot: &BotState,
) -> Result<(), WaitUntilShutdownError> {
    wait_for_signal().await?;
    SHUTDOWN.store(true, Ordering::Relaxed);
    tracing::info!("gracefully shutting down...");

    tracing::debug!("deleting all now playing messages...");
    for data in bot.lavalink().iter_player_data() {
        data.write().await.delete_now_playing_message(bot).await;
    }

    tracing::debug!("sending close frames to all shards...");
    for sender in senders {
        let _ = sender.close(CloseFrame::NORMAL);
    }

    tracing::debug!("killing all shard gateway event handlers...");
    for jh in tasks {
        let _ = jh.await;
    }

    tracing::info!("shut down gracefully");
    Ok(())
}
