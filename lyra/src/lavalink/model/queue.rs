use std::{collections::VecDeque, num::NonZeroUsize};

use lavalink_rs::model::track::TrackData;
use rayon::iter::{IntoParallelIterator, ParallelExtend, ParallelIterator};
use tokio::sync::Notify;
use twilight_model::id::{marker::UserMarker, Id};

use super::queue_indexer::{Indexer, IndexerType};

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
            Self::Off => "➡️",
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

    pub const fn data(&self) -> &TrackData {
        &self.track
    }

    pub fn into_data(self) -> TrackData {
        self.track
    }
}

pub struct Queue {
    inner: VecDeque<Item>,
    index: usize,
    indexer: Indexer,
    repeat_mode: RepeatMode,
    advance_lock: Notify,
}

impl Queue {
    pub(super) const fn new() -> Self {
        Self {
            inner: VecDeque::new(),
            indexer: Indexer::Standard,
            index: 0,
            repeat_mode: RepeatMode::Off,
            advance_lock: Notify::const_new(),
        }
    }

    fn position_from(&self, current: Option<&Item>) -> NonZeroUsize {
        let d = usize::from(current.is_some() || self.index == 0);
        // SAFETY: `self.index + d` is non-zero
        unsafe { NonZeroUsize::new_unchecked(self.index + d) }
    }

    pub fn position(&self) -> NonZeroUsize {
        self.position_from(self.current())
    }

    pub const fn index(&self) -> usize {
        self.index
    }

    pub fn index_mut(&mut self) -> &mut usize {
        &mut self.index
    }

    pub fn current_index(&self) -> Option<usize> {
        match self.indexer {
            Indexer::Standard => Some(self.index),
            Indexer::Fair(ref indexer) => indexer.current(self.index),
            Indexer::Shuffled(ref indexer) => indexer.current(self.index),
        }
    }

    pub fn current(&self) -> Option<&Item> {
        self.inner.get(self.current_index()?)
    }

    pub fn current_and_position(&self) -> (Option<&Item>, NonZeroUsize) {
        let current = self.current();
        let position = self.position_from(current);
        (current, position)
    }

    pub fn enqueue(&mut self, tracks: Vec<TrackData>, requester: Id<UserMarker>) {
        match self.indexer {
            Indexer::Fair(ref mut indexer) => indexer.enqueue(tracks.len(), requester),
            Indexer::Shuffled(ref mut indexer) => indexer.enqueue(tracks.len(), self.index),
            Indexer::Standard => {}
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

    pub fn downgrade_repeat_mode(&mut self) {
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
                self.indexer = Indexer::Standard;
            }
            (IndexerType::Standard | IndexerType::Shuffled, IndexerType::Fair) => {
                self.indexer = Indexer::Fair(super::queue_indexer::FairIndexer::new(
                    self.inner.iter(),
                    self.index,
                ));
            }
            (IndexerType::Standard | IndexerType::Fair, IndexerType::Shuffled) => {
                self.indexer = Indexer::Shuffled(super::queue_indexer::ShuffledIndexer::new(
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

    pub fn acquire_advance_lock(&self) {
        tracing::trace!("acquired queue advance lock");
        self.advance_lock.notify_one();
    }

    pub async fn not_advance_locked(&self) -> bool {
        let future = self.advance_lock.notified();
        let duration = crate::core::r#const::misc::QUEUE_ADVANCE_LOCKED_TIMEOUT;
        tokio::time::timeout(duration, future).await.is_err()
    }

    pub fn iter_positions_and_items(
        &self,
    ) -> impl DoubleEndedIterator<Item = (NonZeroUsize, &Item)> + Clone {
        self.iter()
            .enumerate()
            .filter_map(|(i, t)| NonZeroUsize::new(i + 1).map(|i| (i, t)))
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

impl std::ops::Index<NonZeroUsize> for Queue {
    type Output = Item;

    fn index(&self, index: NonZeroUsize) -> &Self::Output {
        &self.inner[index.get() - 1]
    }
}
