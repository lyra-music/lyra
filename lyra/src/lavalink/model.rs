mod connection;
mod correct_info;
mod now_playing;
mod pitch;
mod queue;
mod queue_indexer;

use std::{
    collections::HashMap,
    num::NonZeroU16,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use connection::ConnectionHead;
use futures::FutureExt;
use lavalink_rs::{
    client::LavalinkClient,
    error::LavalinkResult,
    model::{player::ConnectionInfo, track::TrackInfo},
    player_context::PlayerContext,
};
use lyra_ext::time::track_timestamp::TrackTimestamp;
use moka::future::Cache;
use sqlx::{Pool, Postgres};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, broadcast, mpsc, oneshot, watch};
use twilight_cache_inmemory::InMemoryCache;
use twilight_http::Client;
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker, MessageMarker},
};

use crate::{
    command::{
        poll::Poll as PlayerPoll,
        require::{InVoice, PartialInVoice},
    },
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

type Response<T> = oneshot::Sender<Result<T, UnrecognisedConnection>>;

enum Instruction {
    /// Insert a new connection
    Insert(Id<GuildMarker>, Connection),
    /// Remove a connection
    Remove(Id<GuildMarker>),
    /// Query if a connection exists
    Exists(Id<GuildMarker>, oneshot::Sender<bool>),
    /// Dispatch an event to a connection
    Dispatch(Id<GuildMarker>, Event, Response<()>),
    /// Subscribe to events from a connection
    Subscribe(Id<GuildMarker>, Response<broadcast::Receiver<Event>>),
    /// Notify a connection of a change
    NotifyChange(Id<GuildMarker>, Response<()>),
    /// Subscribe to changes from a connection
    SubscribeOnChange(Id<GuildMarker>, Response<watch::Receiver<()>>),
    /// Toggle mute
    ///
    /// Returns the current mute state if successful
    ToggleMute(Id<GuildMarker>, Response<bool>),
    /// Set mute
    SetMute(Id<GuildMarker>, bool, Response<()>),
    /// Set the channel for a connection
    SetChannel(Id<GuildMarker>, Id<ChannelMarker>, Response<()>),
    /// Set the text channel for a connection
    SetTextChannel(Id<GuildMarker>, Id<ChannelMarker>, Response<()>),
    /// Get basic connection info
    Head(Id<GuildMarker>, Response<ConnectionHead>),
    /// Get the connection poll info
    GetPoll(Id<GuildMarker>, Response<Option<PlayerPoll>>),
    /// Set the connection poll info
    SetPoll(Id<GuildMarker>, Option<PlayerPoll>, Response<()>),
}

/// The result of a future that waits for a value to be sent
pub struct Awaitable<T> {
    receiver: oneshot::Receiver<T>,
}

impl<T> Future for Awaitable<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.receiver.poll_unpin(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(_)) => panic!("Actor sent no result (This is a bug)"),
            Poll::Pending => Poll::Pending,
        }
    }
}

struct ConnectionsActor {
    connections: HashMap<Id<GuildMarker>, Connection>,
    receiver: mpsc::UnboundedReceiver<Instruction>,
}

impl ConnectionsActor {
    pub fn new(receiver: mpsc::UnboundedReceiver<Instruction>) -> Self {
        Self {
            connections: HashMap::new(),
            receiver,
        }
    }

    fn with_connection<T>(
        &self,
        guild_id: Id<GuildMarker>,
        sender: oneshot::Sender<Result<T, UnrecognisedConnection>>,
        f: impl FnOnce(&Connection) -> T,
    ) {
        if let Some(connection) = self.connections.get(&guild_id) {
            let result = f(connection);
            let _ = sender.send(Ok(result));
        } else {
            let _ = sender.send(Err(UnrecognisedConnection));
        }
    }

    fn with_connection_mut<T>(
        &mut self,
        guild_id: Id<GuildMarker>,
        sender: oneshot::Sender<Result<T, UnrecognisedConnection>>,
        f: impl FnOnce(&mut Connection) -> T,
    ) {
        if let Some(connection) = self.connections.get_mut(&guild_id) {
            let result = f(connection);
            let _ = sender.send(Ok(result));
        } else {
            let _ = sender.send(Err(UnrecognisedConnection));
        }
    }

    pub async fn run(&mut self) {
        while let Some(instruction) = self.receiver.recv().await {
            match instruction {
                Instruction::Insert(guild_id, connection) => {
                    self.connections.insert(guild_id, connection);
                }
                Instruction::Remove(guild_id) => {
                    self.connections.remove(&guild_id);
                }
                Instruction::Exists(guild_id, sender) => {
                    let exists = self.connections.contains_key(&guild_id);
                    let _ = sender.send(exists);
                }
                Instruction::Dispatch(guild_id, event, sender) => {
                    self.with_connection(guild_id, sender, |c| c.dispatch(event));
                }
                Instruction::Subscribe(guild_id, sender) => {
                    self.with_connection(guild_id, sender, Connection::subscribe);
                }
                Instruction::NotifyChange(guild_id, sender) => {
                    self.with_connection(guild_id, sender, Connection::notify_change);
                }
                Instruction::SubscribeOnChange(guild_id, sender) => {
                    self.with_connection(guild_id, sender, Connection::subscribe_on_changed);
                }
                Instruction::ToggleMute(guild_id, sender) => {
                    self.with_connection_mut(guild_id, sender, |c| {
                        c.mute = !c.mute;
                        c.mute
                    });
                }
                Instruction::SetMute(guild_id, mute, sender) => {
                    self.with_connection_mut(guild_id, sender, |c| {
                        c.mute = mute;
                    });
                }
                Instruction::SetChannel(guild_id, channel_id, sender) => {
                    self.with_connection_mut(guild_id, sender, |c| {
                        c.channel_id = channel_id;
                    });
                }
                Instruction::SetTextChannel(guild_id, channel_id, sender) => {
                    self.with_connection_mut(guild_id, sender, |c| {
                        c.text_channel_id = channel_id;
                    });
                }
                Instruction::Head(guild_id, sender) => {
                    self.with_connection(guild_id, sender, |c| c.into());
                }
                Instruction::GetPoll(guild_id, sender) => {
                    self.with_connection(guild_id, sender, |c| c.poll().cloned());
                }
                Instruction::SetPoll(id, poll, sender) => {
                    self.with_connection_mut(id, sender, |c| {
                        c.set_poll(poll);
                    });
                }
            }
        }
    }
}

/// Represents a connection to the lavalink server
pub struct ConnectionHandle<'a> {
    parent: &'a Lavalink,
    guild_id: Id<GuildMarker>,
}

impl ConnectionHandle<'_> {
    fn send_instruction(&self, instruction: Instruction) {
        self.parent
            .sender
            .as_ref()
            .expect("Lavalink was not started")
            .send(instruction)
            .expect("Lavalink instruction sender must not be closed");
    }

    fn call_awaitable<T>(
        &self,
        f: impl FnOnce(Response<T>) -> Instruction,
    ) -> Awaitable<Result<T, UnrecognisedConnection>> {
        let (sender, receiver) = oneshot::channel();
        self.send_instruction(f(sender));
        Awaitable { receiver }
    }

    async fn call_result<T>(
        &self,
        f: impl FnOnce(Response<T>) -> Instruction,
    ) -> Result<T, UnrecognisedConnection> {
        let (sender, receiver) = oneshot::channel();
        self.send_instruction(f(sender));
        receiver
            .await
            .expect("Lavalink connection sender must not be closed")
    }

    pub fn dispatch(&self, event: Event) -> Awaitable<Result<(), UnrecognisedConnection>> {
        self.call_awaitable(|sender| Instruction::Dispatch(self.guild_id, event, sender))
    }

    pub fn notify_change(&self) -> Awaitable<Result<(), UnrecognisedConnection>> {
        self.call_awaitable(|sender| Instruction::NotifyChange(self.guild_id, sender))
    }

    pub fn toggle_mute(&self) -> Awaitable<Result<bool, UnrecognisedConnection>> {
        self.call_awaitable(|sender| Instruction::ToggleMute(self.guild_id, sender))
    }

    pub fn set_mute(&self, mute: bool) -> Awaitable<Result<(), UnrecognisedConnection>> {
        self.call_awaitable(|sender| Instruction::SetMute(self.guild_id, mute, sender))
    }

    pub fn set_channel(
        &self,
        channel_id: Id<ChannelMarker>,
    ) -> Awaitable<Result<(), UnrecognisedConnection>> {
        self.call_awaitable(|sender| Instruction::SetChannel(self.guild_id, channel_id, sender))
    }

    pub fn set_text_channel(
        &self,
        channel_id: Id<ChannelMarker>,
    ) -> Awaitable<Result<(), UnrecognisedConnection>> {
        self.call_awaitable(|sender| Instruction::SetTextChannel(self.guild_id, channel_id, sender))
    }

    pub fn set_poll(&self, poll: PlayerPoll) -> Awaitable<Result<(), UnrecognisedConnection>> {
        self.call_awaitable(|sender| Instruction::SetPoll(self.guild_id, Some(poll), sender))
    }

    pub fn reset_poll(&self) -> Awaitable<Result<(), UnrecognisedConnection>> {
        self.call_awaitable(|sender| Instruction::SetPoll(self.guild_id, None, sender))
    }

    pub async fn get_poll(&self) -> Result<Option<PlayerPoll>, UnrecognisedConnection> {
        self.call_result(|sender| Instruction::GetPoll(self.guild_id, sender))
            .await
    }

    pub async fn get_head(&self) -> Result<ConnectionHead, UnrecognisedConnection> {
        self.call_result(|sender| Instruction::Head(self.guild_id, sender))
            .await
    }

    pub async fn subscribe(&self) -> Result<broadcast::Receiver<Event>, UnrecognisedConnection> {
        self.call_result(|sender| Instruction::Subscribe(self.guild_id, sender))
            .await
    }

    pub async fn subscribe_on_change(&self) -> Result<watch::Receiver<()>, UnrecognisedConnection> {
        self.call_result(|sender| Instruction::SubscribeOnChange(self.guild_id, sender))
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
        let mut lava = Self {
            inner: value,
            sender: None,
        };
        lava.start();
        lava
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
                    e.guild_id
                        .expect("bots should currently only be able to join guild voice channels"),
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
        self.data().expect("player data must exists")
    }
}

impl UnwrappedData for LavalinkClient {
    type Data = OwnedClientData;
    fn data_unwrapped(&self) -> Self::Data {
        self.data().expect("lavalink data must exists")
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

pub trait ConnectionAware: GuildIdAware {}

impl ConnectionAware for InVoice<'_> {}
impl ConnectionAware for PartialInVoice {}

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
