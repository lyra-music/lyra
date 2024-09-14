mod emoji;
mod interaction;

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

use dashmap::DashMap;
use sqlx::{Pool, Postgres};
use tokio::sync::OnceCell;
use twilight_cache_inmemory::InMemoryCache;
use twilight_gateway::ShardId;
use twilight_http::Client;
use twilight_model::{
    guild::{Emoji, Permissions},
    id::{
        marker::{ApplicationMarker, UserMarker},
        Id,
    },
    user::CurrentUser,
};
use twilight_standby::Standby;

use crate::{error::core::DeserializeBodyFromHttpError, lavalink::Lavalink, LavalinkAware};

pub use self::interaction::{
    AcknowledgementAware, Client as InteractionClient, Interface as InteractionInterface,
    MessageResponse, UnitFollowupResult, UnitRespondResult,
};

pub struct Config {
    pub token: &'static str,
    pub lavalink_host: &'static str,
    pub lavalink_pwd: &'static str,
    pub database_url: &'static str,
}

struct GuildCounter {
    total: AtomicUsize,
    counters: DashMap<ShardId, usize>,
}

impl GuildCounter {
    pub fn new() -> Self {
        Self {
            total: AtomicUsize::new(0),
            counters: DashMap::new(),
        }
    }

    pub fn total(&self) -> usize {
        self.total.load(Ordering::Relaxed)
    }

    pub fn reset(&self, shard_id: ShardId, guild_count: usize) {
        let old_shard_guild_count = self.counters.get(&shard_id).map_or(0, |v| *v);

        self.counters.insert(shard_id, guild_count);
        let _ = self
            .total
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                (guild_count - old_shard_guild_count != 0)
                    .then_some(n + guild_count - old_shard_guild_count)
            });
    }

    pub fn increment(&self, shard_id: ShardId) {
        self.counters.entry(shard_id).and_modify(|v| *v += 1);
        self.total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement(&self, shard_id: ShardId) {
        self.counters.entry(shard_id).and_modify(|v| *v -= 1);
        self.total.fetch_sub(1, Ordering::Relaxed);
    }
}

pub struct BotInfo {
    started: Instant,
    guild_counter: GuildCounter,
}

impl BotInfo {
    pub fn total_guild_count(&self) -> usize {
        self.guild_counter.total()
    }

    pub fn reset_guild_count(&self, shard_id: ShardId, guild_count: usize) {
        self.guild_counter.reset(shard_id, guild_count);
    }

    pub fn increment_guild_count(&self, shard_id: ShardId) {
        self.guild_counter.increment(shard_id);
    }

    pub fn decrement_guild_count(&self, shard_id: ShardId) {
        self.guild_counter.decrement(shard_id);
    }
}

pub type BotStateRef<'a> = &'a BotState;
pub type OwnedBotState = Arc<BotState>;

pub trait AuthorIdAware {
    fn author_id(&self) -> Id<UserMarker>;
}

pub trait AuthorPermissionsAware {
    fn author_permissions(&self) -> Permissions;
}

pub trait BotStateAware {
    fn bot(&self) -> BotStateRef;
}

pub trait OwnedBotStateAware: BotStateAware {
    fn bot_owned(&self) -> OwnedBotState;
}

pub trait CacheAware {
    fn cache(&self) -> &InMemoryCache;
}

pub trait HttpAware {
    fn http(&self) -> &Client;
}

pub trait DatabaseAware {
    fn db(&self) -> &Pool<Postgres>;
}

pub struct BotState {
    cache: Arc<InMemoryCache>,
    http: Arc<Client>,
    db: Pool<Postgres>,
    standby: Standby,
    lavalink: Lavalink,
    info: BotInfo,
    application_id: OnceCell<Id<ApplicationMarker>>,
    application_emojis: OnceCell<&'static [Emoji]>,
}

impl BotState {
    pub fn new(
        db: Pool<Postgres>,
        http: Arc<Client>,
        cache: Arc<InMemoryCache>,
        lavalink: Lavalink,
    ) -> Self {
        let info = BotInfo {
            started: Instant::now(),
            guild_counter: GuildCounter::new(),
        };

        Self {
            cache,
            http,
            standby: Standby::new(),
            lavalink,
            db,
            info,
            application_id: OnceCell::new(),
            application_emojis: OnceCell::new(),
        }
    }

    pub const fn standby(&self) -> &Standby {
        &self.standby
    }

    pub const fn info(&self) -> &BotInfo {
        &self.info
    }

    pub async fn application_id(
        &self,
    ) -> Result<Id<ApplicationMarker>, DeserializeBodyFromHttpError> {
        self.application_id
            .get_or_try_init(|| async {
                let application = self.http.current_user_application().await?.model().await?;
                Ok(application.id)
            })
            .await
            .copied()
    }

    pub async fn application_emojis(
        &self,
    ) -> Result<&'static [Emoji], DeserializeBodyFromHttpError> {
        self.application_emojis
            .get_or_try_init(|| async {
                let application_id = self.application_id().await?;
                let req = self.http.get_application_emojis(application_id);
                Ok(&*req.await?.models().await?.leak())
            })
            .await
            .copied()
    }

    pub async fn interaction(&self) -> Result<InteractionClient, DeserializeBodyFromHttpError> {
        let client = self.http.interaction(self.application_id().await?);
        Ok(InteractionClient::new(client))
    }

    pub fn user(&self) -> CurrentUser {
        self.cache
            .current_user()
            .unwrap_or_else(|| panic!("current user isn't in cache"))
    }

    #[inline]
    pub fn user_id(&self) -> Id<UserMarker> {
        self.user().id
    }
}

impl LavalinkAware for BotState {
    fn lavalink(&self) -> &Lavalink {
        &self.lavalink
    }
}

impl CacheAware for BotState {
    fn cache(&self) -> &InMemoryCache {
        &self.cache
    }
}

impl CacheAware for Arc<BotState> {
    fn cache(&self) -> &InMemoryCache {
        &self.cache
    }
}

impl HttpAware for BotState {
    fn http(&self) -> &Client {
        &self.http
    }
}

impl DatabaseAware for BotState {
    fn db(&self) -> &Pool<Postgres> {
        &self.db
    }
}
