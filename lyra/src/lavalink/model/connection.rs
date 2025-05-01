use lyra_ext::nested_transpose::NestedTranspose;
use tokio::sync::{broadcast, watch};
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, UserMarker},
};

use crate::{command::poll::Poll, core::r#const};

#[derive(Debug)]
pub struct Connection {
    pub channel_id: Id<ChannelMarker>,
    pub text_channel_id: Id<ChannelMarker>,
    pub mute: bool,
    poll: Option<Poll>,
    change: watch::Sender<()>,
    event_sender: broadcast::Sender<Event>,
}

impl Connection {
    pub fn new(channel_id: Id<ChannelMarker>, text_channel_id: Id<ChannelMarker>) -> Self {
        let (change, _) = watch::channel(());

        Self {
            channel_id,
            text_channel_id,
            mute: false,
            change,
            event_sender: broadcast::channel(16).0,
            poll: None,
        }
    }

    /// Wait until the connection is changed or the timeout is reached.
    pub fn subscribe_on_changed(&self) -> watch::Receiver<()> {
        self.change.subscribe()
    }

    pub const fn poll(&self) -> Option<&Poll> {
        self.poll.as_ref()
    }

    pub const fn set_poll(&mut self, poll: Option<Poll>) {
        self.poll = poll;
    }

    /// Dispatch an event to all subscribers of this connection.
    pub fn dispatch(&self, event: Event) {
        let _ = self.event_sender.send(event);
    }

    /// Notify the connection to trigger a change.
    pub fn notify_change(&self) {
        tracing::trace!("notified connection change");
        self.change.send(()).ok();
    }

    /// Subscribe to events from this connection.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.event_sender.subscribe()
    }
}

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

#[allow(clippy::needless_pass_by_ref_mut)] // false positive
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
