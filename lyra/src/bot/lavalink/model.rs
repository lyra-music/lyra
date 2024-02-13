use std::{
    collections::{HashSet, VecDeque},
    fmt::Display,
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Range, RangeBounds},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use futures::Future;
use itertools::Itertools;
use lavalink_rs::{
    client::LavalinkClient,
    error::LavalinkResult,
    model::{player::ConnectionInfo, track::TrackData},
    player_context::PlayerContext,
};
use rand::{seq::SliceRandom, Rng};
use rayon::prelude::{IntoParallelIterator, ParallelExtend, ParallelIterator};
use tokio::sync::{broadcast, Notify, RwLock};
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker, MessageMarker, UserMarker},
    Id,
};

use crate::bot::{
    command::poll::Poll,
    core::r#const::misc::WAIT_FOR_BOT_EVENTS_TIMEOUT,
    error::{component::queue::remove::WithAdvanceLockAndStoppedError, lavalink::ProcessResult},
    ext::util::{chunked_range, multi_interleave, NestedTranspose},
};

pub trait ClientAware {
    fn lavalink(&self) -> &Lavalink;
}

pub trait Process {
    async fn process(self) -> ProcessResult;
}

#[derive(Hash, Copy, Clone)]
pub enum RepeatMode {
    Off,
    All,
    Track,
}

impl RepeatMode {
    pub const fn next(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::Track,
            Self::Track => Self::Off,
        }
    }

    pub const fn emoji(&self) -> &str {
        match self {
            Self::Off => "**` 🡲 `**",
            Self::All => "🔁",
            Self::Track => "🔂",
        }
    }
    pub const fn description(&self) -> &str {
        match self {
            Self::Off => "Disabled Repeat.",
            Self::All => "Repeating the entire queue.",
            Self::Track => "Repeating only the current track.",
        }
    }
}

impl Display for RepeatMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

pub struct QueueItem {
    track: TrackData,
    requester: Id<UserMarker>,
}

impl QueueItem {
    const fn new(track: TrackData, requester: Id<UserMarker>) -> Self {
        Self { track, requester }
    }

    pub const fn requester(&self) -> Id<UserMarker> {
        self.requester
    }

    pub const fn track(&self) -> &TrackData {
        &self.track
    }

    pub fn into_track(self) -> TrackData {
        self.track
    }
}

pub enum QueueIndexerType {
    Standard,
    Fair,
    Shuffled,
}

enum QueueIndexer {
    Standard,
    Fair(FairIndexer),
    Shuffled(ShuffledIndexer),
}

impl QueueIndexer {
    const fn kind(&self) -> QueueIndexerType {
        match self {
            Self::Standard => QueueIndexerType::Standard,
            Self::Fair(_) => QueueIndexerType::Fair,
            Self::Shuffled(_) => QueueIndexerType::Shuffled,
        }
    }

    fn dequeue(&mut self, indices: impl Iterator<Item = usize>) {
        match self {
            Self::Fair(indexer) => indexer.dequeue_or_drain(indices),
            Self::Shuffled(indexer) => indexer.dequeue(&indices.collect()),
            Self::Standard => {}
        }
    }

    fn drain(&mut self, range: impl RangeBounds<usize> + Iterator<Item = usize>) {
        match self {
            Self::Fair(indexer) => indexer.dequeue_or_drain(range),
            Self::Shuffled(indexer) => indexer.drain(range),
            Self::Standard => {}
        }
    }

    fn clear(&mut self) {
        match self {
            Self::Fair(indexer) => indexer.clear(),
            Self::Shuffled(indexer) => indexer.clear(),
            Self::Standard => {}
        }
    }
}

struct FairIndexer {
    starting_index: usize,
    inner: Vec<(Id<UserMarker>, usize)>,
}

impl FairIndexer {
    fn new<'a>(items: impl Iterator<Item = &'a QueueItem>, starting_index: usize) -> Self {
        let inner = items
            .skip(starting_index)
            .group_by(|c| c.requester)
            .into_iter()
            .map(|(r, g)| (r, g.count()))
            .collect();

        Self {
            starting_index,
            inner,
        }
    }

    fn iter_bucket_lens(&self) -> impl Iterator<Item = usize> + '_ {
        self.inner.iter().map(|(_, l)| l).copied()
    }

    fn iter_bucket_ranges(&self) -> impl Iterator<Item = Range<usize>> + '_ {
        self.inner.iter().scan(self.starting_index, |i, (_, l)| {
            let j = *i;
            *i += l;
            Some(j..*i)
        })
    }

    fn iter_indices(&self) -> impl Iterator<Item = usize> + '_ {
        multi_interleave(
            chunked_range(self.starting_index, self.iter_bucket_lens().collect())
                .map(IntoIterator::into_iter)
                .collect(),
        )
    }

    fn current(&self, current_index: usize) -> Option<usize> {
        self.iter_indices().nth(current_index - self.starting_index)
    }

    fn enqueue(&mut self, additional: usize, requester: Id<UserMarker>) {
        match self.inner.last_mut() {
            Some((last_requester, last_size)) if *last_requester == requester => {
                *last_size += additional;
            }
            _ => self.inner.push((requester, additional)),
        }
    }

    fn dequeue_or_drain(&mut self, mut indices: impl Iterator<Item = usize>) {
        let bucket_ranges = self.iter_bucket_ranges().collect::<Box<_>>();
        let mut iter_bucket_ranges = bucket_ranges.iter().peekable();
        self.inner.retain_mut(|(_, l)| {
            if indices
                .next()
                .is_some_and(|i| iter_bucket_ranges.peek().is_some_and(|r| r.contains(&i)))
            {
                *l -= 1;
                iter_bucket_ranges.next();
            }
            *l == 0
        });
    }

    fn clear(&mut self) {
        self.inner.clear();
        self.starting_index = 0;
    }
}

struct ShuffledIndexer(Vec<usize>);

impl ShuffledIndexer {
    fn new(size: usize, starting_index: usize) -> Self {
        let mut rest = (0..size).collect::<Vec<_>>();
        let mut next = rest.drain(starting_index + 1..).collect::<Vec<_>>();
        next.shuffle(&mut rand::thread_rng());
        rest.extend(next);

        Self(rest)
    }

    fn current(&self, current_index: usize) -> Option<usize> {
        self.0.get(current_index).copied()
    }

    fn enqueue(&mut self, additional: usize, current_index: usize) {
        let old_len = self.0.len();
        self.0.reserve(additional);

        let mut rng = rand::thread_rng();
        (0..additional)
            .map(|d| rng.gen_range(current_index + 1..=old_len + d))
            .zip(old_len..old_len + additional)
            .for_each(|(i, e)| self.0.insert(i, e));
    }

    fn dequeue(&mut self, indices: &HashSet<usize>) {
        self.0.retain(|i| !indices.contains(i));
    }

    fn drain(&mut self, range: impl RangeBounds<usize>) {
        self.0.drain(range);
    }

    fn clear(&mut self) {
        self.0.clear();
    }
}

pub struct Queue {
    inner: VecDeque<QueueItem>,
    index: usize,
    indexer: QueueIndexer,
    repeat_mode: RepeatMode,
    advance_lock: AtomicBool,
    current_track_started: u64,
}

impl Deref for Queue {
    type Target = VecDeque<QueueItem>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Queue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Queue {
    pub const fn new() -> Self {
        Self {
            inner: VecDeque::new(),
            indexer: QueueIndexer::Standard,
            index: 0,
            repeat_mode: RepeatMode::Off,
            advance_lock: AtomicBool::new(false),
            current_track_started: 0,
        }
    }

    pub fn position(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.index + self.current().map_or(0, |_| 1))
            .expect("`self.index + 1` must be nonzero")
    }

    pub const fn index(&self) -> &usize {
        &self.index
    }

    pub fn index_mut(&mut self) -> &mut usize {
        &mut self.index
    }

    pub fn advance_locked(&self) -> bool {
        self.advance_lock.load(Ordering::SeqCst)
    }

    pub fn advance_lock(&self) {
        self.advance_lock.store(true, Ordering::SeqCst);
    }

    pub fn advance_unlock(&self) {
        self.advance_lock.store(false, Ordering::Relaxed);
    }

    pub fn current_index(&self) -> Option<usize> {
        match self.indexer {
            QueueIndexer::Standard => Some(self.index),
            QueueIndexer::Fair(ref indexer) => indexer.current(self.index),
            QueueIndexer::Shuffled(ref indexer) => indexer.current(self.index),
        }
    }

    pub fn current(&self) -> Option<&QueueItem> {
        self.inner.get(self.current_index()?)
    }

    pub fn current_and_index(&self) -> Option<(&QueueItem, usize)> {
        self.current_index()
            .and_then(|i| Some((self.inner.get(i)?, i)))
    }

    pub fn enqueue(&mut self, tracks: Vec<TrackData>, requester: Id<UserMarker>) {
        match self.indexer {
            QueueIndexer::Fair(ref mut indexer) => indexer.enqueue(tracks.len(), requester),
            QueueIndexer::Shuffled(ref mut indexer) => indexer.enqueue(tracks.len(), self.index),
            QueueIndexer::Standard => {}
        }
        let queues = tracks.into_par_iter().map(|t| QueueItem::new(t, requester));
        self.inner.par_extend(queues);
    }

    pub fn dequeue<'a>(
        &'a mut self,
        positions: &'a [NonZeroUsize],
    ) -> impl Iterator<Item = QueueItem> + 'a {
        let iter_indices = positions.iter().map(|p| p.get() - 1);
        self.indexer.dequeue(iter_indices.clone());
        iter_indices.rev().filter_map(|i| self.inner.remove(i))
    }

    fn reset(&mut self) {
        self.repeat_mode = RepeatMode::Off;
        self.index = 0;
        self.indexer.clear();
    }

    pub fn drain(
        &mut self,
        indices: impl RangeBounds<usize> + Iterator<Item = usize> + Clone,
    ) -> impl Iterator<Item = QueueItem> + '_ {
        self.indexer.drain(indices.clone());
        self.inner.drain(indices)
    }

    pub fn drain_all(&mut self) -> impl Iterator<Item = QueueItem> + '_ {
        self.reset();
        self.inner.drain(..)
    }

    pub fn clear(&mut self) {
        self.reset();
        self.inner.clear();
    }

    pub const fn repeat_mode(&self) -> RepeatMode {
        self.repeat_mode
    }

    pub fn set_repeat_mode(&mut self, mode: RepeatMode) {
        self.repeat_mode = mode;
    }

    pub fn adjust_repeat_mode(&mut self) {
        if let RepeatMode::All | RepeatMode::Track = self.repeat_mode {
            self.repeat_mode = if self.len() > 1 {
                RepeatMode::All
            } else {
                RepeatMode::Off
            }
        }
    }

    pub const fn indexer_type(&self) -> QueueIndexerType {
        self.indexer.kind()
    }

    pub fn set_indexer_type(&mut self, kind: QueueIndexerType) {
        match (self.indexer.kind(), kind) {
            (QueueIndexerType::Fair | QueueIndexerType::Shuffled, QueueIndexerType::Standard) => {
                self.indexer = QueueIndexer::Standard;
            }
            (QueueIndexerType::Standard | QueueIndexerType::Shuffled, QueueIndexerType::Fair) => {
                self.indexer = QueueIndexer::Fair(FairIndexer::new(self.inner.iter(), self.index));
            }
            (QueueIndexerType::Standard | QueueIndexerType::Fair, QueueIndexerType::Shuffled) => {
                self.indexer = QueueIndexer::Shuffled(ShuffledIndexer::new(self.len(), self.index));
            }
            _ => {}
        }
    }

    pub fn advance(&mut self) {
        match self.repeat_mode {
            RepeatMode::Off => {
                self.index += 1;
            }
            RepeatMode::All => {
                self.index = (self.index + 1) % self.len();
            }
            RepeatMode::Track => {}
        }
    }

    pub async fn stop_with_advance_lock(
        &self,
        guild_id: Id<GuildMarker>,
        lavalink: &Lavalink,
    ) -> Result<(), WithAdvanceLockAndStoppedError> {
        self.with_advance_lock_and_stopped(guild_id, lavalink, |_| async { Ok(()) })
            .await
    }

    pub async fn with_advance_lock_and_stopped<
        F: Future<Output = Result<(), WithAdvanceLockAndStoppedError>> + Send,
    >(
        &self,
        guild_id: Id<GuildMarker>,
        lavalink: &Lavalink,
        f: impl FnOnce(PlayerContext) -> F + Send,
    ) -> Result<(), WithAdvanceLockAndStoppedError> {
        self.advance_lock();

        let player = lavalink.player(guild_id);
        player.stop_now().await?;
        f(player).await?;
        Ok(())
    }
}

pub struct ConnectionData {
    pub channel_id: Id<ChannelMarker>,
    pub text_channel_id: Id<ChannelMarker>,
    poll: Option<Poll>,
    change_notify: Notify,
    event_sender: broadcast::Sender<Event>,
}

impl ConnectionData {
    pub fn new(channel_id: Id<ChannelMarker>, text_channel_id: Id<ChannelMarker>) -> Self {
        Self {
            channel_id,
            text_channel_id,
            change_notify: Notify::new(),
            event_sender: broadcast::channel(16).0,
            poll: None,
        }
    }

    pub async fn just_changed(&self) -> bool {
        tokio::time::timeout(
            Duration::from_millis(WAIT_FOR_BOT_EVENTS_TIMEOUT.into()),
            self.change_notify.notified(),
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
        self.event_sender.send(event).ok();
    }

    pub fn notify_change(&self) {
        self.change_notify.notify_one();
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.event_sender.subscribe()
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    QueueClear,
    QueueRepeat,
    AlternateVoteCast(Id<UserMarker>),
    AlternateVoteDjCast,
    AlternateVoteCastedAlready(crate::bot::command::poll::Vote),
    AlternateVoteCastDenied,
}

pub struct PlayerData {
    pub connection: ConnectionData,
    queue: Queue,
    now_playing_message_id: Option<Id<MessageMarker>>,
}

impl PlayerData {
    pub fn new(channel_id: Id<ChannelMarker>, text_channel_id: Id<ChannelMarker>) -> Self {
        Self {
            connection: ConnectionData::new(channel_id, text_channel_id),
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

    #[allow(clippy::needless_pass_by_ref_mut)]
    pub async fn wait_for_via(
        rx: &mut broadcast::Receiver<Event>,
        predicate: impl Fn(&Event) -> bool + Send + Sync,
    ) -> EventRecvResult<Option<Event>> {
        let event = tokio::time::timeout(
            Duration::from_millis(WAIT_FOR_BOT_EVENTS_TIMEOUT.into()),
            async {
                loop {
                    let event = rx.recv().await?;
                    if predicate(&event) {
                        return Ok(event);
                    }
                }
            },
        );
        Ok(event.await.transpose()?.ok())
    }
}

#[derive(Clone)]
pub struct Lavalink(LavalinkClient);

impl From<LavalinkClient> for Lavalink {
    fn from(value: LavalinkClient) -> Self {
        Self(value)
    }
}

pub type EventRecvResult<T> = Result<T, broadcast::error::RecvError>;

impl Lavalink {
    pub fn process(&self, event: &twilight_gateway::Event) {
        match event {
            twilight_gateway::Event::VoiceServerUpdate(e) => {
                self.0
                    .handle_voice_server_update(e.guild_id, e.token.clone(), e.endpoint.clone());
            }
            twilight_gateway::Event::VoiceStateUpdate(e) => self.0.handle_voice_state_update(
                e.guild_id.expect("guild_id must exist"),
                e.channel_id,
                e.user_id,
                e.session_id.clone(),
            ),
            _ => {}
        }
    }

    async fn connection_info(&self, guild_id: Id<GuildMarker>) -> ConnectionInfo {
        self.0
            .get_connection_info(guild_id, Duration::MAX)
            .await
            .expect("timeout should not have been reached")
    }

    pub async fn new_player_data(
        &self,
        guild_id: Id<GuildMarker>,
        channel_id: Id<ChannelMarker>,
        text_channel_id: Id<ChannelMarker>,
    ) -> LavalinkResult<()> {
        let info = self.connection_info(guild_id).await;
        let data = Arc::new(RwLock::new(PlayerData::new(channel_id, text_channel_id)));
        self.0
            .create_player_context_with_data(guild_id, info, data)
            .await?;

        Ok(())
    }

    pub fn get_player_data(&self, guild_id: Id<GuildMarker>) -> Option<Arc<RwLock<PlayerData>>> {
        self.0
            .get_player_context(guild_id)
            .map(|c| c.data().expect("data type must be valid"))
    }

    pub fn player(&self, guild_id: Id<GuildMarker>) -> PlayerContext {
        self.0
            .get_player_context(guild_id)
            .expect("player context must exist")
    }

    pub fn player_data(&self, guild_id: Id<GuildMarker>) -> Arc<RwLock<PlayerData>> {
        self.player(guild_id)
            .data()
            .expect("data type must be valid")
    }

    pub async fn notify_connection_change(&self, guild_id: Id<GuildMarker>) {
        self.player_data(guild_id)
            .read()
            .await
            .connection
            .notify_change();
    }

    #[inline]
    pub async fn dispatch(&self, guild_id: Id<GuildMarker>, event: Event) {
        self.player_data(guild_id)
            .read()
            .await
            .connection
            .dispatch(event);
    }

    #[inline]
    pub async fn dispatch_queue_clear(&self, guild_id: Id<GuildMarker>) {
        self.dispatch(guild_id, Event::QueueClear).await;
    }
}

impl Deref for Lavalink {
    type Target = LavalinkClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
