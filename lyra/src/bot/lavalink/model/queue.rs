use std::{
    collections::VecDeque,
    num::NonZeroUsize,
    sync::atomic::{AtomicBool, Ordering},
};

use futures::Future;
use lavalink_rs::{model::track::TrackData, player_context::PlayerContext};
use rayon::iter::{IntoParallelIterator, ParallelExtend, ParallelIterator};
use twilight_model::id::{marker::UserMarker, Id};

use crate::bot::error::component::queue::remove::WithAdvanceLockAndStoppedError;

use super::queue_indexer::{IndexerType, QueueIndexer};

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

impl std::fmt::Display for RepeatMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.description())
    }
}

#[derive(Debug)]
pub struct Item {
    track: TrackData,
    pub(super) requester: Id<UserMarker>,
}

impl Item {
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

pub struct Queue {
    inner: VecDeque<Item>,
    index: usize,
    indexer: QueueIndexer,
    repeat_mode: RepeatMode,
    advance_lock: AtomicBool,
    current_track_started: u64,
}

impl Queue {
    pub(super) const fn new() -> Self {
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
        let d = usize::from(self.current().is_some() || self.index == 0);
        // SAFETY: `self.index + d` is non-zero
        unsafe { NonZeroUsize::new_unchecked(self.index + d) }
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

    pub fn current(&self) -> Option<&Item> {
        self.inner.get(self.current_index()?)
    }

    pub fn current_and_index(&self) -> Option<(&Item, usize)> {
        self.current_index()
            .and_then(|i| Some((self.inner.get(i)?, i)))
    }

    pub fn enqueue(&mut self, tracks: Vec<TrackData>, requester: Id<UserMarker>) {
        match self.indexer {
            QueueIndexer::Fair(ref mut indexer) => indexer.enqueue(tracks.len(), requester),
            QueueIndexer::Shuffled(ref mut indexer) => indexer.enqueue(tracks.len(), self.index),
            QueueIndexer::Standard => {}
        }
        let queues = tracks.into_par_iter().map(|t| Item::new(t, requester));
        self.inner.par_extend(queues);
    }

    pub fn dequeue<'a>(
        &'a mut self,
        positions: &'a [NonZeroUsize],
    ) -> impl Iterator<Item = Item> + 'a {
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
        indices: impl std::ops::RangeBounds<usize> + Iterator<Item = usize> + Clone,
    ) -> impl Iterator<Item = Item> + '_ {
        self.indexer.drain(indices.clone());
        self.inner.drain(indices)
    }

    pub fn drain_all(&mut self) -> impl Iterator<Item = Item> + '_ {
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

    pub const fn indexer_type(&self) -> IndexerType {
        self.indexer.kind()
    }

    pub fn set_indexer_type(&mut self, kind: IndexerType) {
        match (self.indexer.kind(), kind) {
            (IndexerType::Fair | IndexerType::Shuffled, IndexerType::Standard) => {
                self.indexer = QueueIndexer::Standard;
            }
            (IndexerType::Standard | IndexerType::Shuffled, IndexerType::Fair) => {
                self.indexer = QueueIndexer::Fair(super::queue_indexer::FairIndexer::new(
                    self.inner.iter(),
                    self.index,
                ));
            }
            (IndexerType::Standard | IndexerType::Fair, IndexerType::Shuffled) => {
                self.indexer = QueueIndexer::Shuffled(super::queue_indexer::ShuffledIndexer::new(
                    self.len(),
                    self.index,
                ));
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
        player: &PlayerContext,
    ) -> Result<(), WithAdvanceLockAndStoppedError> {
        self.with_advance_lock_and_stopped(player, |_| async { Ok(()) })
            .await
    }

    pub async fn with_advance_lock_and_stopped<'a, 'b, F>(
        &self,
        player: &'a PlayerContext,
        f: impl FnOnce(&'b PlayerContext) -> F + Send,
    ) -> Result<(), WithAdvanceLockAndStoppedError>
    where
        F: Future<Output = Result<(), WithAdvanceLockAndStoppedError>> + Send,
        'a: 'b,
    {
        self.advance_lock();

        player.stop_now().await?;
        f(player).await?;
        Ok(())
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub fn remove(&mut self, index: usize) -> Option<Item> {
        self.inner.remove(index)
    }

    #[inline]
    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, Item> {
        self.inner.iter()
    }

    #[inline]
    pub fn insert(&mut self, index: usize, value: Item) {
        self.inner.insert(index, value);
    }
}

impl std::ops::Index<usize> for Queue {
    type Output = Item;

    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index]
    }
}
