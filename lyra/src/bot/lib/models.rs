use std::{env, net::SocketAddr, str::FromStr, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use hyper::client::{Client as HyperClient, HttpConnector};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::{Latency, MessageSender, Shard};
use twilight_http::{
    client::{ClientBuilder, InteractionClient},
    Client as HttpClient,
};
use twilight_lavalink::{model::IncomingEvent, node::IncomingEvents, Lavalink};
use twilight_model::{
    channel::message::AllowedMentions,
    id::{marker::UserMarker, Id},
    oauth::Application,
    user::CurrentUser,
};
use twilight_standby::Standby;

use crate::bot::commands::declare::COMMANDS;

#[derive(Clone)]
pub struct LyraConfig {
    pub token: String,
    pub lavalink_addr: SocketAddr,
    pub lavalink_auth: String,
    pub database_url: String,
}

impl LyraConfig {
    pub fn from_env() -> Self {
        Self {
            token: env::var("BOT_TOKEN").expect("`BOT_TOKEN` must be set"),
            lavalink_addr: SocketAddr::from_str(
                env::var("LAVALINK_ADDR")
                    .expect("`LAVALINK_ADDR` must be set")
                    .as_str(),
            )
            .expect("`LAVALINK_ADDR` must be a valid address"),
            lavalink_auth: env::var("LAVALINK_AUTH").expect("`LAVALINK_AUTH` must be set"),
            database_url: env::var("DATABASE_URL").expect("`DATABASE_URL` must be set"),
        }
    }
}

struct LavalinkComponent {
    client: Lavalink,
    rx: Arc<RwLock<IncomingEvents>>,
}

pub struct Lyra {
    config: LyraConfig,
    cache: InMemoryCache,
    http: HttpClient,
    lavalink: LavalinkComponent,
    db: Pool<Postgres>,
    hyper: HyperClient<HttpConnector>,
    standby: Standby,
    sender: MessageSender,
    latency: Arc<RwLock<Latency>>,
    started: DateTime<Utc>,
}

impl Lyra {
    pub async fn new(config: LyraConfig, shard: Arc<RwLock<Shard>>) -> anyhow::Result<Self> {
        let shard_count = 1u64;

        let LyraConfig {
            token,
            lavalink_addr,
            lavalink_auth,
            database_url,
        } = config.clone();

        let http = ClientBuilder::default()
            .default_allowed_mentions(AllowedMentions::default())
            .token(token)
            .build();
        let user_id = http.current_user().await?.model().await?.id;

        let lavalink_client = Lavalink::new(user_id, shard_count);
        let (_, lavalink_rx) = lavalink_client.add(lavalink_addr, lavalink_auth).await?;

        let sender = shard.read().await.sender();
        let latency = RwLock::new(shard.read().await.latency().clone()).into();

        let db = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await?;

        let lavalink = LavalinkComponent {
            client: lavalink_client,
            rx: RwLock::new(lavalink_rx).into(),
        };

        Ok(Self {
            config,
            cache: InMemoryCache::new(),
            http,
            lavalink,
            db,
            hyper: HyperClient::new(),
            standby: Standby::new(),
            sender,
            latency,
            started: Utc::now(),
        })
    }

    pub fn cache(&self) -> &InMemoryCache {
        &self.cache
    }

    pub const fn http(&self) -> &HttpClient {
        &self.http
    }

    pub const fn hyper(&self) -> &HyperClient<HttpConnector> {
        &self.hyper
    }

    pub const fn lavalink(&self) -> &Lavalink {
        &self.lavalink.client
    }

    pub const fn db(&self) -> &Pool<Postgres> {
        &self.db
    }

    pub const fn standby(&self) -> &Standby {
        &self.standby
    }

    pub const fn sender(&self) -> &MessageSender {
        &self.sender
    }

    pub const fn started(&self) -> &DateTime<Utc> {
        &self.started
    }

    pub fn elapsed(&self) -> Duration {
        Utc::now() - self.started
    }

    pub async fn disconnect_lavalink(&self) {
        self.lavalink.client.disconnect(self.config.lavalink_addr);
    }

    pub async fn next_lavalink_event(&self) -> Option<IncomingEvent> {
        self.lavalink.rx.write().await.next().await
    }

    pub async fn latency(&self) -> Latency {
        self.latency.read().await.clone()
    }

    pub async fn update_latency(&self, latency: Latency) {
        *self.latency.write().await = latency;
    }

    pub async fn app(&self) -> anyhow::Result<Application> {
        Ok(self.http.current_user_application().await?.model().await?)
    }

    pub async fn interaction_client(&self) -> anyhow::Result<InteractionClient> {
        Ok(self.http.interaction(self.app().await?.id))
    }

    pub async fn register_app_commands(&self) -> anyhow::Result<()> {
        let client = self.interaction_client().await?;

        client.set_global_commands(COMMANDS.as_ref()).await?;

        Ok(())
    }

    pub fn user(&self) -> CurrentUser {
        self.cache
            .current_user()
            .expect("current user object must be available")
    }

    #[inline]
    pub fn user_id(&self) -> Id<UserMarker> {
        self.user().id
    }
}
