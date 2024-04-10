mod connection;
mod correct_info;
mod queue;
mod queue_indexer;

use std::{ops::Deref, sync::Arc};

use lavalink_rs::{
    client::LavalinkClient, error::LavalinkResult, model::player::ConnectionInfo,
    player_context::PlayerContext,
};
use tokio::sync::RwLock;
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker, MessageMarker},
    Id,
};

use crate::bot::core::r#const;

use self::connection::{Connection, ConnectionRef, ConnectionRefMut};

pub use self::{
    connection::{wait_for_with, Event, EventRecvResult},
    correct_info::{CorrectPlaylistInfo, CorrectTrackInfo},
    queue::{Item as QueueItem, Queue, RepeatMode},
    queue_indexer::IndexerType,
};

pub trait ClientAware {
    fn lavalink(&self) -> &Lavalink;
}

pub struct PlayerData {
    queue: Queue,
    now_playing_message_id: Option<Id<MessageMarker>>,
}

impl PlayerData {
    pub const fn new() -> Self {
        Self {
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
}

#[derive(Debug)]
pub struct Lavalink {
    inner: LavalinkClient,
    connections: dashmap::DashMap<Id<GuildMarker>, Connection>,
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
    fn _handle_voice_server_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        token: String,
        endpoint: Option<String>,
    );
    fn _handle_voice_state_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        channel_id: Option<impl Into<lavalink_rs::model::ChannelId>>,
        user_id: impl Into<lavalink_rs::model::UserId>,
        session_id: String,
    );
    fn process(&self, event: &twilight_gateway::Event) {
        match event {
            twilight_gateway::Event::VoiceServerUpdate(e) => {
                self._handle_voice_server_update(e.guild_id, e.token.clone(), e.endpoint.clone());
            }
            twilight_gateway::Event::VoiceStateUpdate(e) => {
                self._handle_voice_state_update(
                    e.guild_id.expect("event received in a guild"),
                    e.channel_id,
                    e.user_id,
                    e.session_id.clone(),
                );
            }
            _ => {}
        }
    }

    async fn _get_connection_info(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        timeout: std::time::Duration,
    ) -> LavalinkResult<ConnectionInfo>;

    async fn _create_player_context_with_data<Data: std::any::Any + Send + Sync>(
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
            ._get_connection_info(
                guild_id,
                *r#const::connection::GET_LAVALINK_CONNECTION_INFO_TIMEOUT,
            )
            .await?;
        tracing::trace!("getting lavalink connection info took {:?}", now.elapsed());

        let data = Arc::new(RwLock::new(PlayerData::new()));
        self._create_player_context_with_data(guild_id, info, data)
            .await?;

        Ok(())
    }

    fn _get_player_context(&self, guild_id: impl Into<LavalinkGuildId>) -> Option<PlayerContext>;
    fn get_player_data(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
    ) -> Option<Arc<RwLock<PlayerData>>> {
        self._get_player_context(guild_id)
            .map(|c| c.data().expect("data type is valid"))
    }
    fn player(&self, guild_id: impl Into<LavalinkGuildId> + Send) -> PlayerContext {
        self._get_player_context(guild_id)
            .expect("player context exists")
    }
    fn player_data(&self, guild_id: impl Into<LavalinkGuildId> + Send) -> Arc<RwLock<PlayerData>> {
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
}

impl DelegateMethods for LavalinkClient {
    #[inline]
    fn _handle_voice_server_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        token: String,
        endpoint: Option<String>,
    ) {
        self.handle_voice_server_update(guild_id, token, endpoint);
    }

    #[inline]
    fn _handle_voice_state_update(
        &self,
        guild_id: impl Into<LavalinkGuildId>,
        channel_id: Option<impl Into<lavalink_rs::model::ChannelId>>,
        user_id: impl Into<lavalink_rs::model::UserId>,
        session_id: String,
    ) {
        self.handle_voice_state_update(guild_id, channel_id, user_id, session_id);
    }

    #[inline]
    async fn _get_connection_info(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        timeout: std::time::Duration,
    ) -> LavalinkResult<ConnectionInfo> {
        self.get_connection_info(guild_id, timeout).await
    }

    #[inline]
    async fn _create_player_context_with_data<Data: std::any::Any + Send + Sync>(
        &self,
        guild_id: impl Into<LavalinkGuildId> + Send,
        connection_info: impl Into<ConnectionInfo> + Send,
        user_data: Arc<Data>,
    ) -> LavalinkResult<PlayerContext> {
        self.create_player_context_with_data(guild_id, connection_info, user_data)
            .await
    }

    #[inline]
    fn _get_player_context(&self, guild_id: impl Into<LavalinkGuildId>) -> Option<PlayerContext> {
        self.get_player_context(guild_id)
    }
}

impl Deref for Lavalink {
    type Target = LavalinkClient;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
