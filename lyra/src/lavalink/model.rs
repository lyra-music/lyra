mod connection;
mod correct_info;
mod now_playing;
mod pitch;
mod queue;
mod queue_indexer;

use std::{num::NonZeroU16, sync::Arc, time::Duration};

use lavalink_rs::{
    client::LavalinkClient,
    error::LavalinkResult,
    model::{player::ConnectionInfo, track::TrackInfo},
    player_context::PlayerContext,
};
use lyra_ext::time::track_timestamp::TrackTimestamp;
use moka::future::Cache;
use sqlx::{Pool, Postgres};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use twilight_cache_inmemory::InMemoryCache;
use twilight_http::Client;
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker, MessageMarker},
};

use crate::{
    command::require::{InVoice, PartialInVoice},
    core::{
        r#const,
        model::{CacheAware, DatabaseAware, HttpAware, OwnedHttpAware},
    },
    error::{
        UnrecognisedConnection,
        lavalink::{NewNowPlayingMessageError, UpdateNowPlayingMessageError},
    },
    gateway::GuildIdAware,
};

use self::connection::{ConnectionRef, ConnectionRefMut};

pub use self::{
    connection::{Connection, Event, EventRecvResult, wait_for_with},
    correct_info::{CorrectPlaylistInfo, CorrectTrackInfo},
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

    fn get_connection(&self) -> Option<ConnectionRef> {
        self.lavalink().get_connection(self.guild_id())
    }

    fn get_connection_mut(&self) -> Option<ConnectionRefMut> {
        self.lavalink().get_connection_mut(self.guild_id())
    }

    /// # Errors
    /// when an unrecognised connection was found
    fn try_get_connection(&self) -> Result<ConnectionRef, UnrecognisedConnection> {
        self.lavalink().try_get_connection(self.guild_id())
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
            // SAFETY: `100` is non-zero
            volume: unsafe { NonZeroU16::new_unchecked(100) },
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
    pub async fn new_now_playing_message(
        &mut self,
        http: Arc<Client>,
        data: NowPlayingData,
    ) -> Result<(), NewNowPlayingMessageError> {
        self.now_playing_message =
            Some(NowPlayingMessage::new(http, data, self.text_channel_id).await?);
        Ok(())
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
}

pub struct Lavalink {
    inner: LavalinkClient,
    connections: dashmap::DashMap<Id<GuildMarker>, Connection>,
}

impl DelegateMethods for Lavalink {
    #[inline]
    fn handle_voice_server_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        token: String,
        endpoint: Option<String>,
    ) {
        <LavalinkClient as DelegateMethods>::handle_voice_server_update(
            &self.inner,
            guild_id,
            token,
            endpoint,
        );
    }

    #[inline]
    fn handle_voice_state_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        channel_id: Option<impl Into<lavalink_rs::model::ChannelId>>,
        user_id: impl Into<lavalink_rs::model::UserId>,
        session_id: String,
    ) {
        <LavalinkClient as DelegateMethods>::handle_voice_state_update(
            &self.inner,
            guild_id,
            channel_id,
            user_id,
            session_id,
        );
    }

    #[inline]
    async fn get_connection_info(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        timeout: std::time::Duration,
    ) -> LavalinkResult<ConnectionInfo> {
        <LavalinkClient as DelegateMethods>::get_connection_info(&self.inner, guild_id, timeout)
            .await
    }

    #[inline]
    async fn create_player_context_with_data<Data: std::any::Any + Send + Sync>(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        connection_info: impl Into<ConnectionInfo> + Send,
        user_data: Arc<Data>,
    ) -> LavalinkResult<PlayerContext> {
        <LavalinkClient as DelegateMethods>::create_player_context_with_data(
            &self.inner,
            guild_id,
            connection_info,
            user_data,
        )
        .await
    }

    #[inline]
    fn get_player_context(&self, guild_id: impl Into<LavalinkGuildId>) -> Option<PlayerContext> {
        <LavalinkClient as DelegateMethods>::get_player_context(&self.inner, guild_id)
    }
}

impl From<LavalinkClient> for Lavalink {
    fn from(value: LavalinkClient) -> Self {
        Self {
            inner: value,
            connections: dashmap::DashMap::new(),
        }
    }
}

type LavalinkGuildId = lavalink_rs::model::GuildId;

pub trait DelegateMethods {
    fn handle_voice_server_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        token: String,
        endpoint: Option<String>,
    );
    fn handle_voice_state_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        channel_id: Option<impl Into<lavalink_rs::model::ChannelId>>,
        user_id: impl Into<lavalink_rs::model::UserId>,
        session_id: String,
    );
    fn process(&self, event: &twilight_gateway::Event) {
        match event {
            twilight_gateway::Event::VoiceServerUpdate(e) => {
                self.handle_voice_server_update(e.guild_id, e.token.clone(), e.endpoint.clone());
            }
            twilight_gateway::Event::VoiceStateUpdate(e) => {
                self.handle_voice_state_update(
                    // SAFETY: this bot cannot join DM voice calls,
                    //         meaning all voice states will be from a guild voice channel,
                    //         so `e.guild_id` is present
                    unsafe { e.guild_id.unwrap_unchecked() },
                    e.channel_id,
                    e.user_id,
                    e.session_id.clone(),
                );
            }
            _ => {}
        }
    }

    async fn get_connection_info(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        timeout: std::time::Duration,
    ) -> LavalinkResult<ConnectionInfo>;
    async fn get_connection_info_traced(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
    ) -> LavalinkResult<ConnectionInfo> {
        let now = tokio::time::Instant::now();
        let info = self
            .get_connection_info(
                guild_id,
                r#const::connection::GET_LAVALINK_CONNECTION_INFO_TIMEOUT,
            )
            .await?;
        tracing::trace!("getting lavalink connection info took {:?}", now.elapsed());
        Ok(info)
    }

    async fn create_player_context_with_data<Data: std::any::Any + Send + Sync>(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        connection_info: impl Into<ConnectionInfo> + Send,
        user_data: Arc<Data>,
    ) -> LavalinkResult<PlayerContext>;
    async fn new_player(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send + Copy,
        channel_id: Id<ChannelMarker>,
    ) -> LavalinkResult<PlayerContext> {
        let info = self.get_connection_info_traced(guild_id).await?;
        let data = Arc::new(RwLock::new(RawPlayerData::new(channel_id)));
        let player = self
            .create_player_context_with_data(guild_id, info, data)
            .await?;

        Ok(player)
    }

    fn get_player_context(&self, guild_id: impl Into<LavalinkGuildId>) -> Option<PlayerContext>;
    fn get_player_data(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
    ) -> Option<OwnedPlayerData> {
        self.get_player_context(guild_id)
            .map(|c| c.data_unwrapped())
    }
}

impl Lavalink {
    pub fn clone_inner(&self) -> LavalinkClient {
        self.inner.clone()
    }

    pub fn new_connection_with(&self, guild_id: Id<GuildMarker>, connection: Connection) {
        self.connections.insert(guild_id, connection);
    }

    pub fn drop_connection(&self, guild_id: Id<GuildMarker>) {
        self.connections.remove(&guild_id);
    }

    pub fn get_connection(&self, guild_id: Id<GuildMarker>) -> Option<ConnectionRef> {
        self.connections.get(&guild_id)
    }

    #[inline]
    pub fn try_get_connection(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> Result<ConnectionRef, UnrecognisedConnection> {
        self.get_connection(guild_id).ok_or(UnrecognisedConnection)
    }

    #[inline]
    pub fn try_get_connection_mut(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> Result<ConnectionRefMut, UnrecognisedConnection> {
        self.get_connection_mut(guild_id)
            .ok_or(UnrecognisedConnection)
    }

    pub fn connection_from(&self, cx: &impl GetConnection) -> ConnectionRef {
        // SAFETY: because the caller has an instance of `InVoice`,
        //         this proves that there is a voice connection currently.
        unsafe { self.connections.get(&cx.guild_id()).unwrap_unchecked() }
    }

    pub fn connection_mut_from(&self, cx: &impl GetConnection) -> ConnectionRefMut {
        // SAFETY: because the caller has an instance of `InVoice`,
        //         this proves that there is a voice connection currently.
        unsafe { self.connections.get_mut(&cx.guild_id()).unwrap_unchecked() }
    }

    pub fn get_connection_mut(&self, guild_id: Id<GuildMarker>) -> Option<ConnectionRefMut> {
        self.connections.get_mut(&guild_id)
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

impl DelegateMethods for LavalinkClient {
    #[inline]
    fn handle_voice_server_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        token: String,
        endpoint: Option<String>,
    ) {
        self.handle_voice_server_update(guild_id, token, endpoint);
    }

    #[inline]
    fn handle_voice_state_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        channel_id: Option<impl Into<lavalink_rs::model::ChannelId>>,
        user_id: impl Into<lavalink_rs::model::UserId>,
        session_id: String,
    ) {
        self.handle_voice_state_update(guild_id, channel_id, user_id, session_id);
    }

    #[inline]
    async fn get_connection_info(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        timeout: std::time::Duration,
    ) -> LavalinkResult<ConnectionInfo> {
        self.get_connection_info(guild_id, timeout).await
    }

    #[inline]
    async fn create_player_context_with_data<Data: std::any::Any + Send + Sync>(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        connection_info: impl Into<ConnectionInfo> + Send,
        user_data: Arc<Data>,
    ) -> LavalinkResult<PlayerContext> {
        self.create_player_context_with_data(guild_id, connection_info, user_data)
            .await
    }

    #[inline]
    fn get_player_context(&self, guild_id: impl Into<LavalinkGuildId>) -> Option<PlayerContext> {
        self.get_player_context(guild_id)
    }
}

pub trait UnwrappedData {
    type Data;
    fn data_unwrapped(&self) -> Self::Data;
}

impl UnwrappedData for PlayerContext {
    type Data = OwnedPlayerData;
    fn data_unwrapped(&self) -> Self::Data {
        // SAFETY: Player data exists of type `Arc<RwLock<PlayerData>>`
        unsafe { self.data().unwrap_unchecked() }
    }
}

impl UnwrappedData for LavalinkClient {
    type Data = OwnedClientData;
    fn data_unwrapped(&self) -> Self::Data {
        // SAFETY: Lavalink data exists of type `Arc<ClientData>`
        unsafe { self.data().unwrap_unchecked() }
    }
}

pub trait UnwrappedPlayerInfoUri {
    fn into_uri_unwrapped(self) -> String;
    fn uri_unwrapped(&self) -> &str;
}

impl UnwrappedPlayerInfoUri for TrackInfo {
    fn uri_unwrapped(&self) -> &str {
        self.uri
            .as_ref()
            .unwrap_or_else(|| panic!("local tracks are unsupported"))
    }

    fn into_uri_unwrapped(self) -> String {
        self.uri
            .unwrap_or_else(|| panic!("local tracks are unsupported"))
    }
}

pub trait GetConnection: GuildIdAware {}

impl GetConnection for InVoice<'_> {}
impl GetConnection for PartialInVoice {}

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
