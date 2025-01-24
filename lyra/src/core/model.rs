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
use twilight_cache_inmemory::{model::CachedMember, InMemoryCache};
use twilight_gateway::ShardId;
use twilight_http::Client;
use twilight_model::{
    guild::{Emoji, PartialMember, Permissions},
    id::{
        marker::{ApplicationMarker, UserMarker},
        Id,
    },
    user::{CurrentUser, User},
    util::ImageHash,
};
use twilight_standby::Standby;

use crate::{error::core::DeserialiseBodyFromHttpError, lavalink::Lavalink, LavalinkAware};

pub use self::interaction::{
    AcknowledgementAware, Client as InteractionClient, Interface as InteractionInterface,
    MessageResponse, UnitFollowupResult, UnitRespondResult,
};

use super::r#static::application;

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

pub trait DiscriminatorAware {
    fn discriminator(&self) -> u16;
}

pub trait UserIdAware {
    fn user_id(&self) -> Id<UserMarker>;
}

impl UserIdAware for CachedMember {
    fn user_id(&self) -> Id<UserMarker> {
        self.user_id()
    }
}

pub trait UserPermissionsAware {
    fn user_permissions(&self) -> Permissions;
}

pub trait AvatarAware {
    fn avatar(&self) -> Option<ImageHash>;
}

pub trait UserAware {
    fn user(&self) -> &User;
}

impl UserAware for User {
    fn user(&self) -> &User {
        self
    }
}

impl<T: UserAware> UserIdAware for T {
    fn user_id(&self) -> Id<UserMarker> {
        self.user().id
    }
}

impl<T: UserAware> AvatarAware for T {
    fn avatar(&self) -> Option<ImageHash> {
        self.user().avatar
    }
}

impl<T: UserAware> DiscriminatorAware for T {
    fn discriminator(&self) -> u16 {
        self.user().discriminator
    }
}

pub trait PartialMemberAware {
    fn member(&self) -> &PartialMember;
}

impl PartialMemberAware for PartialMember {
    fn member(&self) -> &PartialMember {
        self
    }
}

pub trait GuildAvatarAware {
    fn guild_avatar(&self) -> Option<ImageHash>;
}

impl GuildAvatarAware for CachedMember {
    fn guild_avatar(&self) -> Option<ImageHash> {
        self.avatar()
    }
}

impl<T: PartialMemberAware> GuildAvatarAware for T {
    fn guild_avatar(&self) -> Option<ImageHash> {
        self.member().avatar
    }
}

pub trait UserNickAware {
    fn nick(&self) -> Option<&str>;
}

impl<T: PartialMemberAware> UserNickAware for T {
    fn nick(&self) -> Option<&str> {
        self.member().nick.as_deref()
    }
}

pub trait UsernameAware {
    fn username(&self) -> &str;
}

impl<T: UserAware> UsernameAware for T {
    fn username(&self) -> &str {
        self.user().name.as_str()
    }
}

pub trait UserGlobalNameAware {
    fn user_global_name(&self) -> Option<&str>;
}

impl<T: UserAware> UserGlobalNameAware for T {
    fn user_global_name(&self) -> Option<&str> {
        self.user().global_name.as_deref()
    }
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

pub trait OwnedHttpAware {
    fn http_owned(&self) -> Arc<Client>;
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
        }
    }

    pub const fn standby(&self) -> &Standby {
        &self.standby
    }

    pub const fn info(&self) -> &BotInfo {
        &self.info
    }

    #[inline]
    pub async fn application_id(
        &self,
    ) -> Result<Id<ApplicationMarker>, DeserialiseBodyFromHttpError> {
        application::id(self).await
    }

    #[inline]
    pub async fn application_emojis(
        &self,
    ) -> Result<&'static [Emoji], DeserialiseBodyFromHttpError> {
        application::emojis(self).await
    }

    pub async fn interaction(&self) -> Result<InteractionClient, DeserialiseBodyFromHttpError> {
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
