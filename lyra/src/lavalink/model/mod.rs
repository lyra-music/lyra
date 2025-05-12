mod connection;
mod correct_info;
mod delegate;
mod now_playing;
mod pitch;
mod queue;
mod queue_indexer;

use std::{
    num::{NonZeroU16, NonZeroUsize},
    sync::Arc,
    time::Duration,
};

use connection::{Awaitable, ConnectionHandle, ConnectionsActor, Instruction};
use lavalink_rs::{
    client::LavalinkClient, error::LavalinkResult, model::track::TrackInfo,
    player_context::PlayerContext,
};
use lyra_ext::time::track_timestamp::TrackTimestamp;
use moka::future::Cache;
use sqlx::{Pool, Postgres};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, mpsc, oneshot};
use twilight_cache_inmemory::InMemoryCache;
use twilight_http::Client;
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker, MessageMarker},
};

use crate::{
    core::model::{CacheAware, DatabaseAware, HttpAware, OwnedHttpAware},
    error::{
        UnrecognisedConnection,
        lavalink::{NewNowPlayingMessageError, UpdateNowPlayingMessageError},
    },
    gateway::GuildIdAware,
};

pub use self::{
    connection::{Connection, Event, EventRecvResult, wait_for_with},
    correct_info::{CorrectPlaylistInfo, CorrectTrackInfo},
    delegate::DelegateMethods,
    now_playing::{
        Data as NowPlayingData, Message as NowPlayingMessage, Update as NowPlayingDataUpdate,
    },
    pitch::Pitch,
    queue::{Item as QueueItem, Queue, RepeatMode},
    queue_indexer::IndexerType,
};

pub type PlayerData = RwLock<RawPlayerData>;
pub type OwnedPlayerData = Arc<PlayerData>;
pub type PlayerDataRead<'a> = RwLockReadGuard<'a, RawPlayerData>;
pub type PlayerDataWrite<'a> = RwLockWriteGuard<'a, RawPlayerData>;

pub type OwnedClientData = Arc<ClientData>;
pub trait ClientAware {
    fn lavalink(&self) -> &Lavalink;
}

pub trait ClientAndGuildIdAware: ClientAware + GuildIdAware {
    fn get_player(&self) -> Option<PlayerContext> {
        self.lavalink().get_player_context(self.guild_id())
    }

    fn get_player_data(&self) -> Option<OwnedPlayerData> {
        self.get_player().map(|player| player.data_unwrapped())
    }

    #[expect(async_fn_in_trait)]
    async fn has_connection(&self) -> bool {
        self.lavalink().has_connection(self.guild_id()).await
    }

    fn notify_change(&self) -> Awaitable<Result<(), UnrecognisedConnection>> {
        self.lavalink().handle_for(self.guild_id()).notify_change()
    }

    fn get_conn(&self) -> ConnectionHandle {
        self.lavalink().handle_for(self.guild_id())
    }
}

impl<T> ClientAndGuildIdAware for T where T: ClientAware + GuildIdAware {}

type ClientRefAndGuildId<'a> = (&'a Lavalink, Id<GuildMarker>);

impl ClientAware for ClientRefAndGuildId<'_> {
    fn lavalink(&self) -> &Lavalink {
        self.0
    }
}

impl GuildIdAware for ClientRefAndGuildId<'_> {
    fn guild_id(&self) -> Id<GuildMarker> {
        self.1
    }
}

pub struct RawPlayerData {
    queue: Queue,
    volume: NonZeroU16,
    pitch: Pitch,
    track_timestamp: TrackTimestamp,
    text_channel_id: Id<ChannelMarker>,
    now_playing_message: Option<NowPlayingMessage>,
}

pub type UpdateNowPlayingMessageResult = Result<(), UpdateNowPlayingMessageError>;

impl RawPlayerData {
    pub fn new(text_channel_id: Id<ChannelMarker>) -> Self {
        Self {
            text_channel_id,
            volume: NonZeroU16::new(100).expect("100 must be non-zero"),
            pitch: Pitch::new(),
            queue: Queue::new(),
            track_timestamp: TrackTimestamp::new(),
            now_playing_message: None,
        }
    }

    pub const fn queue(&self) -> &Queue {
        &self.queue
    }

    #[inline]
    pub fn reset_track_timestamp(&mut self) {
        self.track_timestamp.reset();
    }

    #[inline]
    pub const fn queue_mut(&mut self) -> &mut Queue {
        &mut self.queue
    }

    pub const fn volume(&self) -> NonZeroU16 {
        self.volume
    }

    #[inline]
    pub const fn set_volume(&mut self, volume: NonZeroU16) {
        self.volume = volume;
    }

    #[inline]
    pub const fn pitch_mut(&mut self) -> &mut Pitch {
        &mut self.pitch
    }

    pub const fn paused(&self) -> bool {
        self.track_timestamp.paused()
    }

    #[inline]
    pub fn timestamp(&self) -> Duration {
        self.track_timestamp.get()
    }

    #[inline]
    pub fn set_pause(&mut self, state: bool) {
        self.track_timestamp.set_pause(state);
    }

    #[inline]
    pub fn seek_to(&mut self, timestamp: Duration) {
        self.track_timestamp.seek_to(timestamp);
    }

    #[inline]
    pub const fn speed(&self) -> f64 {
        self.track_timestamp.speed()
    }

    #[inline]
    pub fn set_speed(&mut self, multiplier: f64) {
        self.track_timestamp.set_speed(multiplier);
    }

    pub const fn text_channel_id(&self) -> Id<ChannelMarker> {
        self.text_channel_id
    }

    pub const fn set_text_channel_id(&mut self, text_channel_id: Id<ChannelMarker>) {
        self.text_channel_id = text_channel_id;
    }

    pub const fn now_playing_message_id(&self) -> Option<Id<MessageMarker>> {
        match self.now_playing_message {
            Some(ref msg) => Some(msg.id()),
            None => None,
        }
    }

    #[inline]
    pub const fn take_now_playing_message(&mut self) -> Option<NowPlayingMessage> {
        self.now_playing_message.take()
    }

    #[inline]
    pub async fn new_now_playing_message_in(
        &mut self,
        http: Arc<Client>,
        data: NowPlayingData,
        channel_id: Id<ChannelMarker>,
    ) -> Result<(), NewNowPlayingMessageError> {
        self.now_playing_message = Some(NowPlayingMessage::new(http, data, channel_id).await?);
        Ok(())
    }

    #[inline]
    pub async fn new_now_playing_message(
        &mut self,
        http: Arc<Client>,
        data: NowPlayingData,
    ) -> Result<(), NewNowPlayingMessageError> {
        self.new_now_playing_message_in(http, data, self.text_channel_id())
            .await
    }

    pub async fn delete_now_playing_message(&mut self, cx: &(impl HttpAware + Sync)) {
        if let Some(message) = self.take_now_playing_message() {
            let channel_id = message.channel_id();
            let _ = cx.http().delete_message(channel_id, message.id()).await;
        }
    }

    #[inline]
    pub async fn update_and_apply_now_playing_timestamp(
        &mut self,
    ) -> UpdateNowPlayingMessageResult {
        let timestamp = self.timestamp();
        if let Some(ref mut msg) = self.now_playing_message {
            msg.update_timestamp(timestamp);
            msg.apply_update().await?;
        }
        Ok(())
    }

    #[inline]
    async fn update_and_apply_now_playing_data(
        &mut self,
        update: NowPlayingDataUpdate,
    ) -> UpdateNowPlayingMessageResult {
        let timestamp = self.timestamp();
        if let Some(ref mut msg) = self.now_playing_message {
            msg.update(update);
            msg.update_timestamp(timestamp);
            msg.apply_update().await?;
        }
        Ok(())
    }

    #[inline]
    pub async fn set_repeat_mode_then_update_and_apply_to_now_playing(
        &mut self,
        mode: RepeatMode,
    ) -> UpdateNowPlayingMessageResult {
        self.queue_mut().set_repeat_mode(mode);
        self.update_and_apply_now_playing_data(NowPlayingDataUpdate::Repeat(mode))
            .await
    }

    #[inline]
    pub async fn set_indexer_then_update_and_apply_to_now_playing(
        &mut self,
        kind: IndexerType,
    ) -> UpdateNowPlayingMessageResult {
        self.queue_mut().set_indexer_type(kind);
        self.update_and_apply_now_playing_data(NowPlayingDataUpdate::Indexer(kind))
            .await
    }

    #[inline]
    pub async fn update_and_apply_now_playing_pause(
        &mut self,
        paused: bool,
    ) -> UpdateNowPlayingMessageResult {
        self.update_and_apply_now_playing_data(NowPlayingDataUpdate::Paused(paused))
            .await
    }

    #[inline]
    pub async fn update_and_apply_now_playing_queue_len(
        &mut self,
        len: usize,
    ) -> UpdateNowPlayingMessageResult {
        self.update_and_apply_now_playing_data(NowPlayingDataUpdate::QueueLen(len))
            .await
    }

    #[inline]
    pub async fn update_and_apply_now_playing_queue_position(
        &mut self,
        position: NonZeroUsize,
    ) -> UpdateNowPlayingMessageResult {
        self.update_and_apply_now_playing_data(NowPlayingDataUpdate::QueuePosition(position))
            .await
    }
}

pub struct Lavalink {
    inner: LavalinkClient,
    sender: Option<mpsc::UnboundedSender<Instruction>>,
}

impl Lavalink {
    pub fn clone_inner(&self) -> LavalinkClient {
        self.inner.clone()
    }

    pub fn start(&mut self) {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.sender = Some(sender);
        let mut actor = ConnectionsActor::new(receiver);
        tokio::spawn(async move {
            actor.run().await;
        });
    }

    fn send_instruction(&self, instruction: Instruction) {
        self.sender
            .as_ref()
            .expect("Lavalink was not started")
            .send(instruction)
            .expect("Lavalink instruction sender must not be closed");
    }

    #[inline]
    pub const fn handle_for(&self, guild_id: Id<GuildMarker>) -> ConnectionHandle<'_> {
        ConnectionHandle {
            parent: self,
            guild_id,
        }
    }

    pub fn new_connection_with(&self, guild_id: Id<GuildMarker>, connection: Connection) {
        self.send_instruction(Instruction::Insert(guild_id, connection));
    }

    pub fn drop_connection(&self, guild_id: Id<GuildMarker>) {
        self.send_instruction(Instruction::Remove(guild_id));
    }

    pub async fn has_connection(&self, guild_id: Id<GuildMarker>) -> bool {
        let (sender, receiver) = oneshot::channel();
        self.send_instruction(Instruction::Exists(guild_id, sender));
        receiver
            .await
            .expect("Lavalink connection sender must not be closed")
    }

    #[inline]
    pub async fn delete_player(
        &self,
        guild_id: impl Into<lavalink_rs::prelude::GuildId> + Send,
    ) -> LavalinkResult<()> {
        self.inner.delete_player(guild_id).await
    }

    pub fn iter_player_data(&self) -> impl Iterator<Item = OwnedPlayerData> + use<'_> {
        self.inner
            .players
            .iter()
            .filter_map(|p| p.value().0.load().as_ref().map(|ctx| ctx.data_unwrapped()))
    }
}

impl From<LavalinkClient> for Lavalink {
    fn from(value: LavalinkClient) -> Self {
        let mut lava = Self {
            inner: value,
            sender: None,
        };
        lava.start();
        lava
    }
}

pub trait UnwrappedData {
    type Data;
    fn data_unwrapped(&self) -> Self::Data;
}

impl UnwrappedData for PlayerContext {
    type Data = OwnedPlayerData;
    fn data_unwrapped(&self) -> Self::Data {
        self.data().expect("player data must exist")
    }
}

impl UnwrappedData for LavalinkClient {
    type Data = OwnedClientData;
    fn data_unwrapped(&self) -> Self::Data {
        self.data().expect("lavalink data must exist")
    }
}

pub trait UnwrappedPlayerInfoUri {
    fn into_uri_unwrapped(self) -> String;
    fn uri_unwrapped(&self) -> &str;
}

impl UnwrappedPlayerInfoUri for TrackInfo {
    fn uri_unwrapped(&self) -> &str {
        self.uri.as_ref().expect("track must be nonlocal")
    }

    fn into_uri_unwrapped(self) -> String {
        self.uri.expect("track must be nonlocal")
    }
}

pub type ArtworkCache = Cache<(Box<str>, usize), Arc<[u32]>>;

pub struct ClientData {
    db: Pool<Postgres>,
    http: Arc<Client>,
    cache: Arc<InMemoryCache>,
    artwork_cache: ArtworkCache,
}

impl HttpAware for ClientData {
    fn http(&self) -> &Client {
        &self.http
    }
}

impl OwnedHttpAware for ClientData {
    fn http_owned(&self) -> Arc<Client> {
        self.http.clone()
    }
}

impl CacheAware for ClientData {
    fn cache(&self) -> &InMemoryCache {
        &self.cache
    }
}

impl DatabaseAware for ClientData {
    fn db(&self) -> &Pool<Postgres> {
        &self.db
    }
}

impl ClientData {
    pub fn new(http: Arc<Client>, cache: Arc<InMemoryCache>, db: Pool<Postgres>) -> Self {
        Self {
            http,
            cache,
            db,
            artwork_cache: Cache::new(10_000),
        }
    }

    pub const fn artwork_cache(&self) -> &ArtworkCache {
        &self.artwork_cache
    }
}
