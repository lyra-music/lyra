mod connection;
mod correct_info;
mod pitch;
mod queue;
mod queue_indexer;

use std::{num::NonZeroU16, sync::Arc};

use lavalink_rs::{
    client::LavalinkClient, error::LavalinkResult, model::player::ConnectionInfo,
    player_context::PlayerContext,
};
use tokio::sync::RwLock;
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker, MessageMarker},
    Id,
};

use crate::bot::{
    core::r#const,
    gateway::{ExpectedGuildIdAware, GuildIdAware},
};

use self::connection::{Connection, ConnectionRef, ConnectionRefMut};

pub use self::{
    connection::{wait_for_with, Event, EventRecvResult},
    correct_info::{CorrectPlaylistInfo, CorrectTrackInfo},
    pitch::Pitch,
    queue::{Item as QueueItem, Queue, RepeatMode},
    queue_indexer::IndexerType,
};

type PlayerDataRwLockArc = Arc<RwLock<PlayerData>>;

pub trait ClientAware {
    fn lavalink(&self) -> &Lavalink;
}

pub trait PlayerDataAware: ClientAware + GuildIdAware {
    fn get_player_data(&self) -> Option<PlayerDataRwLockArc> {
        self.lavalink().get_player_data(self.get_guild_id()?)
    }
}
pub trait ExpectedPlayerDataAware: ClientAware + ExpectedGuildIdAware {
    fn player_data(&self) -> PlayerDataRwLockArc {
        self.lavalink().player_data(self.guild_id())
    }
}

pub trait PlayerAware: ClientAware + GuildIdAware {
    fn get_player(&self) -> Option<PlayerContext> {
        self.lavalink().get_player_context(self.get_guild_id()?)
    }
}

pub trait ExpectedPlayerAware: ClientAware + ExpectedGuildIdAware {
    fn player(&self) -> PlayerContext {
        self.lavalink().player(self.guild_id())
    }
}

pub struct PlayerData {
    queue: Queue,
    volume: NonZeroU16,
    pitch: Pitch,
    now_playing_message_id: Option<Id<MessageMarker>>,
}

impl PlayerData {
    pub fn new() -> Self {
        Self {
            volume: NonZeroU16::new(100).expect("volume is non-zero"),
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
                    e.guild_id.expect("event received in a guild"),
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
    async fn new_player_data(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send + Copy,
    ) -> LavalinkResult<()> {
        let now = tokio::time::Instant::now();
        let info = self
            .get_connection_info(
                guild_id,
                *r#const::connection::GET_LAVALINK_CONNECTION_INFO_TIMEOUT,
            )
            .await?;
        tracing::trace!("getting lavalink connection info took {:?}", now.elapsed());

        let data = Arc::new(RwLock::new(PlayerData::new()));
        self.create_player_context_with_data(guild_id, info, data)
            .await?;

        Ok(())
    }

    fn get_player_context(&self, guild_id: impl Into<LavalinkGuildId>) -> Option<PlayerContext>;
    fn get_player_data(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
    ) -> Option<PlayerDataRwLockArc> {
        self.get_player_context(guild_id)
            .map(|c| c.data().expect("data type is valid"))
    }
    fn player(&self, guild_id: impl Into<LavalinkGuildId> + Send) -> PlayerContext {
        self.get_player_context(guild_id)
            .expect("player context exists")
    }
    fn player_data(&self, guild_id: impl Into<LavalinkGuildId> + Send) -> PlayerDataRwLockArc {
        self.player(guild_id).data().expect("data type is valid")
    }
}

impl Lavalink {
    pub fn clone_inner(&self) -> LavalinkClient {
        self.inner.clone()
    }

    pub fn new_connection(
        &self,
        guild_id: Id<GuildMarker>,
        channel_id: Id<ChannelMarker>,
        text_channel_id: Id<ChannelMarker>,
    ) {
        self.connections
            .insert(guild_id, Connection::new(channel_id, text_channel_id));
    }

    pub fn drop_connection(&self, guild_id: Id<GuildMarker>) {
        self.connections.remove(&guild_id);
    }

    pub fn get_connection(&self, guild_id: Id<GuildMarker>) -> Option<ConnectionRef> {
        self.connections.get(&guild_id)
    }

    pub fn get_connection_mut(&self, guild_id: Id<GuildMarker>) -> Option<ConnectionRefMut> {
        self.connections.get_mut(&guild_id)
    }

    pub fn connection(&self, guild_id: Id<GuildMarker>) -> ConnectionRef {
        self.get_connection(guild_id).expect("connection exists")
    }

    pub fn connection_mut(&self, guild_id: Id<GuildMarker>) -> ConnectionRefMut {
        self.get_connection_mut(guild_id)
            .expect("connection exists")
    }

    pub fn notify_connection_change(&self, guild_id: Id<GuildMarker>) {
        self.connection(guild_id).notify_change();
    }

    #[inline]
    pub fn dispatch(&self, guild_id: Id<GuildMarker>, event: Event) {
        self.connection(guild_id).dispatch(event);
    }

    #[inline]
    pub fn dispatch_queue_clear(&self, guild_id: Id<GuildMarker>) {
        self.dispatch(guild_id, Event::QueueClear);
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
