use std::{
    env,
    net::SocketAddr,
    ops::Deref,
    str::FromStr,
    sync::{Arc, Mutex},
};

use hyper::client::{Client as HyperClient, HttpConnector};
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::{Latency, MessageSender, Shard};
use twilight_http::{
    client::{ClientBuilder, InteractionClient},
    Client as HttpClient,
};
use twilight_lavalink::Lavalink;
use twilight_model::{channel::message::AllowedMentions, oauth::Application};
use twilight_standby::Standby;

use crate::bot::commands::define::COMMANDS;

pub struct LyraConfig {
    pub token: String,
    pub lavalink_addr: SocketAddr,
    pub lavalink_auth: String,
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
        }
    }

    pub fn as_tuple(&self) -> (String, SocketAddr, String) {
        (
            self.token.clone(),
            self.lavalink_addr.clone(),
            self.lavalink_auth.clone(),
        )
    }
}

pub struct LyraBot {
    cache: InMemoryCache,
    http: HttpClient,
    lavalink: Lavalink,
    hyper: HyperClient<HttpConnector>,
    shard: Arc<Mutex<Shard>>,
    standby: Standby,
}

impl LyraBot {
    pub async fn new(config: &LyraConfig, shard: Arc<Mutex<Shard>>) -> anyhow::Result<Self> {
        let shard_count = 1u64;
        let shard = Arc::clone(&shard);

        let (token, lavalink_addr, lavalink_auth) = config.as_tuple();

        let http = ClientBuilder::default()
            .default_allowed_mentions(AllowedMentions::default())
            .token(token.clone())
            .build();
        let user_id = http.current_user().await?.model().await?.id;

        let lavalink = Lavalink::new(user_id, shard_count);
        lavalink.add(lavalink_addr, lavalink_auth).await?;

        Ok(Self {
            cache: InMemoryCache::new(),
            http,
            lavalink,
            hyper: HyperClient::new(),
            shard,
            standby: Standby::new(),
        })
    }

    pub fn cache(&self) -> &InMemoryCache {
        &self.cache
    }

    pub fn shard(&self) -> Arc<Mutex<Shard>> {
        self.shard.clone()
    }

    pub fn http(&self) -> &HttpClient {
        &self.http
    }

    pub fn hyper(&self) -> &HyperClient<HttpConnector> {
        &self.hyper
    }

    pub fn lavalink(&self) -> &Lavalink {
        &self.lavalink
    }

    pub fn standby(&self) -> &Standby {
        &self.standby
    }

    pub fn sender(&self) -> MessageSender {
        self.shard()
            .lock()
            .expect("another user of `self.shard` must not panick while holding it")
            .sender()
    }

    pub fn latency(&self) -> Latency {
        self.shard()
            .lock()
            .expect("another user of `self.shard` must not panick while holding it")
            .latency()
            .clone()
    }

    pub async fn app(&self) -> anyhow::Result<Application> {
        Ok(self.http.current_user_application().await?.model().await?)
    }

    pub async fn interaction_client(&self) -> anyhow::Result<InteractionClient> {
        Ok(self.http.interaction(self.app().await?.id))
    }

    pub async fn register_app_commands(&self) -> anyhow::Result<()> {
        let commands = COMMANDS
            .iter()
            .map(|(_, c)| c.deref().clone())
            .collect::<Vec<_>>();
        let inter_client = self.interaction_client().await?;

        inter_client.set_global_commands(&commands).await?;

        Ok(())
    }
}
