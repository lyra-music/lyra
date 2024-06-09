use lyra_ext::nested_transpose::NestedTranspose;
use tokio::sync::{broadcast, Notify};
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker, UserMarker},
    Id,
};

use crate::{command::poll::Poll, core::r#const};

#[derive(Debug)]
pub struct Connection {
    pub channel_id: Id<ChannelMarker>,
    pub text_channel_id: Id<ChannelMarker>,
    pub mute: bool,
    poll: Option<Poll>,
    change: Notify,
    event_sender: broadcast::Sender<Event>,
}

pub(super) type ConnectionRef<'a> = dashmap::mapref::one::Ref<'a, Id<GuildMarker>, Connection>;
pub(super) type ConnectionRefMut<'a> =
    dashmap::mapref::one::RefMut<'a, Id<GuildMarker>, Connection>;

impl Connection {
    pub fn new(channel_id: Id<ChannelMarker>, text_channel_id: Id<ChannelMarker>) -> Self {
        Self {
            channel_id,
            text_channel_id,
            mute: false,
            change: Notify::new(),
            event_sender: broadcast::channel(16).0,
            poll: None,
        }
    }

    pub async fn changed(&self) -> bool {
        tokio::time::timeout(
            *r#const::connection::connection_changed_timeout(),
            self.change.notified(),
        )
        .await
        .is_ok()
    }

    pub const fn poll(&self) -> Option<&Poll> {
        self.poll.as_ref()
    }

    pub fn set_poll(&mut self, poll: Poll) {
        self.poll = Some(poll);
    }

    pub fn reset_poll(&mut self) {
        self.poll = None;
    }

    pub fn dispatch(&self, event: Event) {
        let _ = self.event_sender.send(event);
    }

    pub fn notify_change(&self) {
        tracing::trace!("notified connection change");
        self.change.notify_one();
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.event_sender.subscribe()
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

#[allow(clippy::needless_pass_by_ref_mut)]
pub async fn wait_for_with(
    rx: &mut broadcast::Receiver<Event>,
    predicate: impl Fn(&Event) -> bool + Send + Sync,
) -> EventRecvResult<Option<Event>> {
    let event = tokio::time::timeout(*r#const::misc::wait_for_bot_events_timeout(), async {
        loop {
            let event = rx.recv().await?;
            if predicate(&event) {
                return Ok(event);
            }
        }
    });

    Ok(event.await.transpose()?.ok())
}
