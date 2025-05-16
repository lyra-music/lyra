mod ctx_head;
mod followup;
mod response;

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use sqlx::{Pool, Postgres};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot,
};
use twilight_cache_inmemory::{InMemoryCache, model::CachedMember};
use twilight_gateway::ShardId;
use twilight_http::Client;
use twilight_model::{
    guild::{Emoji, PartialMember, Permissions},
    id::{Id, marker::UserMarker},
    user::{CurrentUser, User},
    util::ImageHash,
};
use twilight_standby::Standby;

use crate::{LavalinkAware, error::core::DeserialiseBodyFromHttpError, lavalink::Lavalink};

pub use {
    crate::core::http::Client as InteractionClient,
    ctx_head::CtxHead,
    followup::FollowupTrait as Followup,
    response::{
        Respond, RespondAppCommandModal, RespondAutocomplete, RespondComponent,
        RespondComponentModal, RespondWithDefer, RespondWithDeferUpdate, RespondWithMessage,
        RespondWithModal, RespondWithUpdate,
    },
};

use super::r#static::application;

pub struct Config {
    pub token: &'static str,
    pub lavalink_host: &'static str,
    pub lavalink_pwd: &'static str,
    pub database_url: &'static str,
}

enum CounterOp {
    Increment(ShardId),
    Decrement(ShardId),
    Set(ShardId, usize),
    GetTotal(oneshot::Sender<usize>),
}

struct CounterActor {
    total: usize,
    counters: HashMap<ShardId, usize>,
    receiver: UnboundedReceiver<CounterOp>,
}

impl CounterActor {
    fn new(receiver: UnboundedReceiver<CounterOp>) -> Self {
        Self {
            total: 0,
            counters: HashMap::new(),
            receiver,
        }
    }

    async fn run(&mut self) {
        while let Some(op) = self.receiver.recv().await {
            match op {
                CounterOp::Increment(shard_id) => {
                    *self.counters.entry(shard_id).or_insert(0) += 1;
                    self.total += 1;
                }
                CounterOp::Decrement(shard_id) => {
                    if let Some(count) = self.counters.get_mut(&shard_id) {
                        *count -= 1;
                        self.total -= 1;
                    }
                }
                CounterOp::Set(shard_id, count) => {
                    let old_count = self.counters.insert(shard_id, count);
                    if let Some(old_count) = old_count {
                        self.total += count - old_count;
                    } else {
                        self.total += count;
                    }
                }
                CounterOp::GetTotal(sender) => {
                    let _ = sender.send(self.total);
                }
            }
        }
    }
}

struct GuildCounter {
    sender: Option<UnboundedSender<CounterOp>>,
}

impl GuildCounter {
    pub fn new() -> Self {
        let mut new = Self { sender: None };
        new.start();
        new
    }

    pub fn start(&mut self) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        self.sender = Some(sender);

        let mut actor = CounterActor::new(receiver);
        tokio::spawn(async move {
            actor.run().await;
        });
    }

    fn send_op(&self, op: CounterOp) {
        if let Some(sender) = &self.sender {
            let _ = sender.send(op);
        }
    }

    pub async fn read_total(&self) -> usize {
        let (sender, receiver) = oneshot::channel();
        self.send_op(CounterOp::GetTotal(sender));
        receiver.await.unwrap_or(0)
    }

    pub fn reset(&self, shard_id: ShardId, guild_count: usize) {
        self.send_op(CounterOp::Set(shard_id, guild_count));
    }

    pub fn increment(&self, shard_id: ShardId) {
        self.send_op(CounterOp::Increment(shard_id));
    }

    pub fn decrement(&self, shard_id: ShardId) {
        self.send_op(CounterOp::Decrement(shard_id));
    }
}

pub struct BotInfo {
    started: Instant,
    guild_counter: GuildCounter,
}

impl BotInfo {
    pub fn uptime(&self) -> Duration {
        self.started.elapsed()
    }

    pub async fn total_guild_count(&self) -> usize {
        self.guild_counter.read_total().await
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

impl CacheAware for Arc<InMemoryCache> {
    fn cache(&self) -> &InMemoryCache {
        self
    }
}

pub trait OwnedCacheAware {
    fn cache_owned(&self) -> Arc<InMemoryCache>;
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
    pub async fn application_emojis(
        &self,
    ) -> Result<&'static [Emoji], DeserialiseBodyFromHttpError> {
        application::emojis(self).await
    }

    pub fn interaction(&self) -> InteractionClient {
        InteractionClient::new(self.http.clone())
    }

    pub fn user(&self) -> CurrentUser {
        self.cache
            .current_user()
            .expect("current user must be in cache")
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

impl OwnedCacheAware for BotState {
    fn cache_owned(&self) -> Arc<InMemoryCache> {
        self.cache.clone()
    }
}

impl HttpAware for BotState {
    fn http(&self) -> &Client {
        &self.http
    }
}

impl OwnedHttpAware for BotState {
    fn http_owned(&self) -> Arc<Client> {
        self.http.clone()
    }
}

impl DatabaseAware for BotState {
    fn db(&self) -> &Pool<Postgres> {
        &self.db
    }
}
