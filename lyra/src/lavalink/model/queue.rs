use std::{collections::VecDeque, num::NonZeroUsize, time::Duration};

use lavalink_rs::model::track::TrackData;
use rayon::iter::{IntoParallelIterator, ParallelExtend, ParallelIterator};
use tokio::sync::watch;
use twilight_model::id::{Id, marker::UserMarker};

use super::{
    PlaylistAwareTrackData, PlaylistMetadata,
    queue_indexer::{Indexer, IndexerType},
};

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
            Self::Off => "Disabled repeat",
            Self::All => "Enabled repeat",
            Self::Track => "Enabled repeat one",
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
    track: PlaylistAwareTrackData,
    enqueued: Duration,
    pub(super) requester: Id<UserMarker>,
}

impl Item {
    fn new(track: PlaylistAwareTrackData, requester: Id<UserMarker>) -> Self {
        Self {
            track,
            requester,
            enqueued: lyra_ext::unix_time(),
        }
    }

    pub const fn requester(&self) -> Id<UserMarker> {
        self.requester
    }

    pub const fn data(&self) -> &TrackData {
        self.track.inner()
    }

    pub fn playlist_data(&self) -> Option<&PlaylistMetadata> {
        self.track.playlist()
    }

    pub const fn enqueued(&self) -> Duration {
        self.enqueued
    }

    pub fn into_data(self) -> TrackData {
        self.track.into_inner()
    }
}

pub struct Queue {
    inner: VecDeque<Item>,
    index: usize,
    indexer: Indexer,
    repeat_mode: RepeatMode,
    advancing_enabler: watch::Sender<bool>,
}

impl Queue {
    pub(super) fn new() -> Self {
        Self {
            inner: VecDeque::new(),
            indexer: Indexer::Standard,
            index: 0,
            repeat_mode: RepeatMode::Off,
            advancing_enabler: watch::channel(true).0,
        }
    }

    fn position_from(&self, current: Option<&Item>) -> NonZeroUsize {
        let d = usize::from(current.is_some() || self.index == 0);
        NonZeroUsize::new(self.index + d).expect("normalised queue position must be non-zero")
    }

    pub fn position(&self) -> NonZeroUsize {
        self.position_from(self.current())
    }

    pub const fn index(&self) -> usize {
        self.index
    }

    pub const fn index_mut(&mut self) -> &mut usize {
        &mut self.index
    }

    fn map_index(&self, index: usize) -> Option<usize> {
        match self.indexer {
            Indexer::Standard => Some(index),
            Indexer::Fair(ref indexer) => indexer.current(index),
            Indexer::Shuffled(ref indexer) => indexer.current(index),
        }
    }

    #[inline]
    pub fn map_index_expected(&self, index: usize) -> usize {
        self.map_index(index).expect("track at index exists")
    }

    fn get_mapped(&self, index: usize) -> Option<&Item> {
        self.inner.get(self.map_index(index)?)
    }

    #[inline]
    pub fn current(&self) -> Option<&Item> {
        self.get_mapped(self.index)
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current().map(|_| self.index)
    }

    #[inline]
    pub fn next(&self) -> Option<&Item> {
        self.get_mapped(self.next_index())
    }

    pub fn current_and_position(&self) -> (Option<&Item>, NonZeroUsize) {
        let current = self.current();
        let position = self.position_from(current);
        (current, position)
    }

    pub fn enqueue(&mut self, tracks: Vec<PlaylistAwareTrackData>, requester: Id<UserMarker>) {
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
    ) -> impl Iterator<Item = Item> + use<'a> {
        let iter_indices = positions.iter().map(|p| p.get() - 1);
        self.indexer.dequeue(iter_indices.clone());
        iter_indices.rev().filter_map(|i| self.inner.remove(i))
    }

    fn reset(&mut self) {
        self.repeat_mode = RepeatMode::Off;
        self.index = 0;
        self.indexer.clear();
    }

    pub fn drain<T: std::ops::RangeBounds<usize> + Iterator<Item = usize> + Clone>(
        &mut self,
        indices: T,
    ) -> impl Iterator<Item = Item> + use<'_, T> {
        self.indexer.drain(indices.clone());
        self.inner.drain(indices)
    }

    pub fn drain_all(&mut self) -> impl Iterator<Item = Item> + use<'_> {
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

    pub const fn set_repeat_mode(&mut self, mode: RepeatMode) {
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

    fn next_index(&self) -> usize {
        match self.repeat_mode {
            RepeatMode::Off => self.index + 1,
            RepeatMode::All => (self.index + 1) % self.len(),
            RepeatMode::Track => self.index,
        }
    }

    fn prev_index(&self) -> usize {
        match self.repeat_mode {
            RepeatMode::Off => self.index.saturating_sub(1),
            RepeatMode::All => {
                let len = self.len();
                ((self.index + len).saturating_sub(1)) % len
            }
            RepeatMode::Track => self.index,
        }
    }

    #[inline]
    pub fn advance(&mut self) {
        self.index = self.next_index();
    }

    #[inline]
    pub fn recede(&mut self) {
        self.index = self.prev_index();
    }

    pub fn subscribe_to_advance_enabler(&self) -> watch::Receiver<bool> {
        self.advancing_enabler.subscribe()
    }

    fn set_advancing_state(&self, state: bool) {
        self.advancing_enabler.send_replace(state);
    }

    /// Disables the queue advancing
    ///
    /// # Correctness
    ///
    /// This function must only be called when the current track is ending.
    /// Otherwise, this will lead to incorrect queue advancing behaviour.
    pub fn disable_advancing(&self) {
        tracing::debug!("disabling queue advancing");
        self.set_advancing_state(false);
    }

    pub fn enable_advancing(&self) {
        tracing::debug!("enabling queue advancing");
        self.set_advancing_state(true);
    }

    pub async fn advancing_disabled(&self) -> bool {
        let mut rx = self.subscribe_to_advance_enabler();
        let disabled = tokio::time::timeout(
            crate::core::konst::misc::QUEUE_ADVANCE_DISABLED_TIMEOUT,
            rx.wait_for(|&r| !r),
        )
        .await
        .is_ok()
            || !*rx.borrow();

        if disabled {
            self.enable_advancing();
        }

        disabled
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
