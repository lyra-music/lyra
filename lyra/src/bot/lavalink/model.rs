mod connection;
mod correct_info;
mod pitch;
mod queue;
mod queue_indexer;

use std::{num::NonZeroU16, sync::Arc};

use lavalink_rs::{
    client::LavalinkClient,
    error::LavalinkResult,
    model::{player::ConnectionInfo, track::TrackInfo},
    player_context::PlayerContext,
};
use tokio::sync::RwLock;
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker, MessageMarker},
    Id,
};

use crate::bot::{
    command::require::{CachelessInVoice, InVoice},
    core::r#const,
    gateway::GuildIdAware,
};

use self::connection::{ConnectionRef, ConnectionRefMut};

pub use self::{
    connection::{wait_for_with, Connection, Event, EventRecvResult},
    correct_info::{CorrectPlaylistInfo, CorrectTrackInfo},
    pitch::Pitch,
    queue::{Item as QueueItem, Queue, RepeatMode},
    queue_indexer::IndexerType,
};

pub type PlayerDataRwLockArc = Arc<RwLock<PlayerData>>;

pub trait ClientAware {
    fn lavalink(&self) -> &Lavalink;
}

pub trait PlayerAware: ClientAware + GuildIdAware {
    fn get_player(&self) -> Option<PlayerContext> {
        self.lavalink().get_player_context(self.guild_id())
    }
}

pub struct PlayerData {
    queue: Queue,
    volume: NonZeroU16,
    pitch: Pitch,
    now_playing_message_id: Option<Id<MessageMarker>>,
}

impl PlayerData {
    pub const fn new() -> Self {
        Self {
            // SAFETY: `100` is non-zero
            volume: unsafe { NonZeroU16::new_unchecked(100) },
            pitch: Pitch::new(),
            queue: Queue::new(),
            now_playing_message_id: None,
        }
    }

    pub const fn queue(&self) -> &Queue {
        &self.queue
    }

    pub fn queue_mut(&mut self) -> &mut Queue {
        &mut self.queue
    }

    pub const fn volume(&self) -> NonZeroU16 {
        self.volume
    }

    pub fn set_volume(&mut self, volume: NonZeroU16) {
        self.volume = volume;
    }

    pub fn pitch_mut(&mut self) -> &mut Pitch {
        &mut self.pitch
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

    async fn create_player_context_with_data<Data: std::any::Any + Send + Sync>(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        connection_info: impl Into<ConnectionInfo> + Send,
        user_data: Arc<Data>,
    ) -> LavalinkResult<PlayerContext>;
    async fn new_player(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send + Copy,
    ) -> LavalinkResult<PlayerContext> {
        let now = tokio::time::Instant::now();
        let info = self
            .get_connection_info(
                guild_id,
                *r#const::connection::get_lavalink_connection_info_timeout(),
            )
            .await?;
        tracing::trace!("getting lavalink connection info took {:?}", now.elapsed());

        let data = Arc::new(RwLock::new(PlayerData::new()));
        let player = self
            .create_player_context_with_data(guild_id, info, data)
            .await?;

        Ok(player)
    }

    fn get_player_context(&self, guild_id: impl Into<LavalinkGuildId>) -> Option<PlayerContext>;
    fn get_player_data(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
    ) -> Option<PlayerDataRwLockArc> {
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

    pub fn new_connection(
        &self,
        guild_id: Id<GuildMarker>,
        channel_id: Id<ChannelMarker>,
        text_channel_id: Id<ChannelMarker>,
    ) {
        let connection = Connection::new(channel_id, text_channel_id);
        self.new_connection_with(guild_id, connection);
    }

    pub fn drop_connection(&self, guild_id: Id<GuildMarker>) {
        self.connections.remove(&guild_id);
    }

    pub fn get_connection(&self, guild_id: Id<GuildMarker>) -> Option<ConnectionRef> {
        self.connections.get(&guild_id)
    }

    pub fn connection_from(&self, from: &impl GetConnection) -> ConnectionRef {
        // SAFETY: because the caller has an instance of `InVoice`,
        //         this proves that there is a voice connection currently.
        unsafe { self.connections.get(&from.guild_id()).unwrap_unchecked() }
    }

    pub fn connection_mut_from(&self, from: &impl GetConnection) -> ConnectionRefMut {
        // SAFETY: because the caller has an instance of `InVoice`,
        //         this proves that there is a voice connection currently.
        unsafe {
            self.connections
                .get_mut(&from.guild_id())
                .unwrap_unchecked()
        }
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

pub trait UnwrappedPlayerData {
    fn data_unwrapped(&self) -> PlayerDataRwLockArc;
}

impl UnwrappedPlayerData for PlayerContext {
    fn data_unwrapped(&self) -> PlayerDataRwLockArc {
        // SAFETY: Player data exists of type `Arc<RwLock<PlayerData>>`
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
impl GetConnection for CachelessInVoice {}
