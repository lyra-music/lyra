use std::{
    collections::HashMap,
    pin::Pin,
    task::{Context, Poll},
};

use futures::FutureExt;
use lyra_ext::nested_transpose::NestedTranspose;
use tokio::sync::{broadcast, mpsc, oneshot, watch};
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker, UserMarker},
};

use crate::{command::poll::Poll as PlayerPoll, core::r#const, error::UnrecognisedConnection};

use super::Lavalink;

#[derive(Debug)]
pub struct Connection {
    pub channel_id: Id<ChannelMarker>,
    pub text_channel_id: Id<ChannelMarker>,
    pub mute: bool,
    poll: Option<PlayerPoll>,
    event_sender: broadcast::Sender<Event>,
    /// A watch channel used to enable or disable the voice state update (VSU) handler.
    /// A `true` value indicates the handler should be abled; `false` means disenabled.
    vsu_handler_enabler: watch::Sender<bool>,
}

impl Connection {
    pub fn new(channel_id: Id<ChannelMarker>, text_channel_id: Id<ChannelMarker>) -> Self {
        Self {
            channel_id,
            text_channel_id,
            mute: false,
            poll: None,
            event_sender: broadcast::channel(0xFF).0,
            vsu_handler_enabler: watch::channel(true).0,
        }
    }

    /// Returns a `watch::Receiver` to listen for changes to the voice state update handler's enabled/disabled state.
    fn subscribe_to_vsu_handler_enabler(&self) -> watch::Receiver<bool> {
        self.vsu_handler_enabler.subscribe()
    }

    pub const fn poll(&self) -> Option<&PlayerPoll> {
        self.poll.as_ref()
    }

    pub const fn set_poll(&mut self, poll: Option<PlayerPoll>) {
        self.poll = poll;
    }

    /// Dispatch an event to all subscribers of this connection.
    pub fn dispatch(&self, event: Event) {
        let _ = self.event_sender.send(event);
    }

    /// Updates the current state of the voice state update handler.
    /// `true` enables the handler; `false` disables it.
    fn set_vsu_handler_state(&self, state: bool) {
        self.vsu_handler_enabler.send_replace(state);
    }

    /// Disables the voice state update handler and notifies any subscribers.
    pub fn disable_vsu_handler(&self) {
        tracing::debug!("disabled voice state update handler");
        self.set_vsu_handler_state(false);
    }

    /// Enables the voice state update handler and notifies any subscribers.
    pub fn enable_vsu_handler(&self) {
        tracing::debug!("enabled voice state update handler");
        self.set_vsu_handler_state(true);
    }

    /// Subscribe to events from this connection.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.event_sender.subscribe()
    }
}

#[derive(Clone)]
pub struct ConnectionHead {
    channel_id: Id<ChannelMarker>,
    text_channel_id: Id<ChannelMarker>,
    mute: bool,
}

impl ConnectionHead {
    #[inline]
    pub const fn channel_id(&self) -> Id<ChannelMarker> {
        self.channel_id
    }

    #[inline]
    pub const fn text_channel_id(&self) -> Id<ChannelMarker> {
        self.text_channel_id
    }

    #[inline]
    pub const fn mute(&self) -> bool {
        self.mute
    }
}

impl From<Connection> for ConnectionHead {
    fn from(value: Connection) -> Self {
        Self {
            channel_id: value.channel_id,
            text_channel_id: value.text_channel_id,
            mute: value.mute,
        }
    }
}

impl From<&Connection> for ConnectionHead {
    fn from(value: &Connection) -> Self {
        Self {
            channel_id: value.channel_id,
            text_channel_id: value.text_channel_id,
            mute: value.mute,
        }
    }
}

#[derive(Debug, Clone, const_panic::PanicFmt)]
pub struct AlternateVoteCastUserId(u64);

impl From<Id<UserMarker>> for AlternateVoteCastUserId {
    fn from(value: Id<UserMarker>) -> Self {
        Self(value.get())
    }
}

impl From<AlternateVoteCastUserId> for Id<UserMarker> {
    fn from(value: AlternateVoteCastUserId) -> Self {
        Self::new(value.0)
    }
}

#[derive(Debug, Clone, const_panic::PanicFmt)]
pub enum Event {
    QueueClear,
    QueueRepeat,
    AlternateVoteCast(AlternateVoteCastUserId),
    AlternateVoteDjCast,
    AlternateVoteCastedAlready(crate::command::poll::Vote),
    AlternateVoteCastDenied,
}

pub type EventRecvResult<T> = Result<T, broadcast::error::RecvError>;

pub async fn wait_for_with(
    rx: &mut broadcast::Receiver<Event>,
    predicate: impl Fn(&Event) -> bool + Send + Sync,
) -> EventRecvResult<Option<Event>> {
    let event = tokio::time::timeout(r#const::misc::WAIT_FOR_BOT_EVENTS_TIMEOUT, async {
        loop {
            let event = rx.recv().await?;
            if predicate(&event) {
                return Ok(event);
            }
        }
    });

    Ok(event.await.transpose()?.ok())
}

type Response<T> = oneshot::Sender<Result<T, UnrecognisedConnection>>;

pub(super) enum Instruction {
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
    /// Set the voice state update handler's state for the specified guild.
    /// `true` enables it, `false` disables it.
    SetVsuHandlerState(Id<GuildMarker>, bool, Response<()>),
    /// Subscribe to the voice state update handler's
    /// enable/disable state changes for the specified guild.
    SubscribeToVsuHandlerEnabler(Id<GuildMarker>, Response<watch::Receiver<bool>>),
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

pub(super) struct ConnectionsActor {
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
                Instruction::SetVsuHandlerState(guild_id, reason, sender) => {
                    self.with_connection(guild_id, sender, |c| {
                        c.set_vsu_handler_state(reason);
                    });
                }
                Instruction::SubscribeToVsuHandlerEnabler(guild_id, sender) => {
                    self.with_connection(
                        guild_id,
                        sender,
                        Connection::subscribe_to_vsu_handler_enabler,
                    );
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
    pub(super) parent: &'a Lavalink,
    pub(super) guild_id: Id<GuildMarker>,
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

    /// Issues an instruction to set the voice state update handler's state.
    /// Returns an awaitable result indicating success or unrecognized connection.
    fn set_vsu_handler_state(&self, reason: bool) -> Awaitable<Result<(), UnrecognisedConnection>> {
        self.call_awaitable(|sender| Instruction::SetVsuHandlerState(self.guild_id, reason, sender))
    }

    /// Disables the voice state update handler for the current guild.
    #[inline]
    pub fn disable_vsu_handler(&self) -> Awaitable<Result<(), UnrecognisedConnection>> {
        self.set_vsu_handler_state(false)
    }

    /// Enables the voice state update handler for the current guild.
    #[inline]
    fn enable_vsu_handler(&self) -> Awaitable<Result<(), UnrecognisedConnection>> {
        self.set_vsu_handler_state(true)
    }

    /// Subscribes to the voice state update handler state for this guild.
    /// Returns a receiver to watch for enabled/disabled state changes.
    async fn subscribe_to_vsu_handler_enabler(
        &self,
    ) -> Result<watch::Receiver<bool>, UnrecognisedConnection> {
        self.call_result(|sender| Instruction::SubscribeToVsuHandlerEnabler(self.guild_id, sender))
            .await
    }

    /// Returns `true` if the VSU handler is currently disabled or has changed to disabled recently.
    /// This will attempt to enable it again after detection.
    pub async fn vsu_handler_disabled(&self) -> bool {
        // Attempt to subscribe to state updates.
        let vsu_handler_disabled = if let Ok(mut rx) = self.subscribe_to_vsu_handler_enabler().await
        {
            // Wait up to a timeout duration for the state to change to "disabled".
            tokio::time::timeout(
                crate::core::r#const::connection::CHANGED_TIMEOUT,
                rx.wait_for(|&r| !r),
            )
            .await
            .is_ok()
                || !*rx.borrow() // Check if it's already disabled.
        } else {
            false
        };

        // If the handler is disabled, re-enable it.
        if vsu_handler_disabled {
            // Fire and forget — no need to await.
            self.enable_vsu_handler();
        }

        vsu_handler_disabled
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
}
