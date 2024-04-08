use std::{str::FromStr, sync::Arc};

use dotenvy_macro::dotenv;
use lavalink_rs::{client::LavalinkClient, model::client::NodeDistributionStrategy};
use log::LevelFilter;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions,
};
use tokio::{sync::watch, task::JoinSet};
use twilight_gateway::{
    stream::{self, StartRecommendedError},
    CloseFrame, Config as ShardConfig, Event,
};
use twilight_gateway::{ConfigBuilder, Intents, Shard};
use twilight_http::{client::ClientBuilder, Client};
use twilight_model::{
    channel::message::AllowedMentions,
    gateway::{
        payload::outgoing::update_presence::UpdatePresencePayload,
        presence::{Activity, ActivityType, MinimalActivity, Status},
    },
    id::{marker::UserMarker, Id},
};

use crate::bot::core::r#const::metadata::BANNER;

use super::{
    core::{
        model::{BotState, CacheAware, Config},
        traced,
    },
    error::runner::{StartError, WaitForSignalError, WaitUntilShutdownError},
    gateway,
    lavalink::{self, DelegateMethods, LavalinkAware},
};
use super::{gateway::LastCachedStates, lavalink::Lavalink};

const CONFIG: Config = Config {
    token: dotenv!("BOT_TOKEN"),
    lavalink_host: concat!(dotenv!("SERVER_ADDRESS"), ":", dotenv!("SERVER_PORT")),
    lavalink_pwd: dotenv!("LAVALINK_SERVER_PASSWORD"),
    database_url: dotenv!("DATABASE_URL"),
};
const INTENTS: Intents = Intents::GUILDS.union(Intents::GUILD_VOICE_STATES);

fn build_http_client() -> Client {
    ClientBuilder::default()
        .default_allowed_mentions(AllowedMentions::default())
        .token(CONFIG.token.to_owned())
        .build()
}

fn build_shard_config() -> ShardConfig {
    ConfigBuilder::new(CONFIG.token.to_owned(), INTENTS)
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
        .build()
}

pub(super) async fn start() -> Result<(), StartError> {
    tracing::debug!("began starting the bot");

    let options =
        PgConnectOptions::from_str(CONFIG.database_url)?.log_statements(LevelFilter::Debug);
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    let http = build_http_client();

    let user_id = http.current_user().await?.model().await?.id;
    let lavalink = build_lavalink_client(user_id).await;

    let shards = build_and_split_shards(&http).await?;
    let bot = Arc::new(BotState::new(db, http, lavalink));
    bot.interaction().await?.register_global_commands().await?;

    let (tx, rx) = watch::channel(false);
    let mut set = JoinSet::new();
    for mut shard in shards {
        let mut rx = rx.clone();
        let bot = bot.clone();

        set.spawn(async move {
            tokio::select! {
                () = handle_gateway_events(&mut shard, bot.clone()) => {},
                _ = rx.changed() => {
                    _ = shard.close(CloseFrame::NORMAL).await;
                }
            }
        });
    }

    print_banner();
    Ok(wait_until_shutdown(tx, set).await?)
}

async fn build_and_split_shards(
    client: &Client,
) -> Result<impl Iterator<Item = Shard>, StartRecommendedError> {
    let shard_config = build_shard_config();
    let shards =
        stream::create_recommended(client, shard_config, |_, builder| builder.build()).await?;
    Ok(shards)
}

#[tracing::instrument(skip_all, name = "lavalink")]
async fn build_lavalink_client(user_id: Id<UserMarker>) -> Lavalink {
    let events = lavalink::handlers();

    let nodes = Vec::from([lavalink_rs::node::NodeBuilder {
        hostname: (*CONFIG.lavalink_host).to_string(),
        password: (*CONFIG.lavalink_pwd).to_string(),
        user_id: user_id.into(),
        ..Default::default()
    }]);

    let client = LavalinkClient::new(events, nodes, NodeDistributionStrategy::new()).await;
    client.into()
}

#[tracing::instrument(skip_all, name = "gateway")]
async fn handle_gateway_events(shard: &mut Shard, bot: Arc<BotState>) {
    loop {
        let event = match shard.next_event().await {
            Ok(event) => event,
            Err(source) => {
                tracing::warn!(?source, "error receiving event");

                if source.is_fatal() {
                    break;
                }

                continue;
            }
        };

        process_gateway_events(shard, event, bot.clone());
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

fn print_banner() {
    println!("{}", *BANNER);
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
    tx: watch::Sender<bool>,
    mut set: JoinSet<()>,
) -> Result<(), WaitUntilShutdownError> {
    wait_for_signal().await?;
    tracing::info!("gracefully shutting down...");
    tx.send(true)?;
    while set.join_next().await.is_some() {}
    tracing::info!("shut down gracefully");

    Ok(())
}
