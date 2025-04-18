use std::{collections::HashSet, ops::RangeBounds};

use itertools::Itertools;
use lyra_ext::iter::{chunked_range::chunked_range, multi_interleave::multi_interleave};
use rand::{Rng, seq::SliceRandom};
use twilight_model::id::{Id, marker::UserMarker};

#[derive(Clone, Copy)]
pub enum IndexerType {
    Standard,
    Fair,
    Shuffled,
}

pub(super) enum Indexer {
    Standard,
    Fair(FairIndexer),
    Shuffled(ShuffledIndexer),
}

impl Indexer {
    pub(super) const fn kind(&self) -> IndexerType {
        match self {
            Self::Standard => IndexerType::Standard,
            Self::Fair(_) => IndexerType::Fair,
            Self::Shuffled(_) => IndexerType::Shuffled,
        }
    }

    pub(super) fn dequeue(&mut self, indices: impl Iterator<Item = usize>) {
        match self {
            Self::Fair(indexer) => indexer.dequeue_or_drain(indices),
            Self::Shuffled(indexer) => indexer.dequeue(&indices.collect()),
            Self::Standard => {}
        }
    }

    pub(super) fn drain(&mut self, range: impl RangeBounds<usize> + Iterator<Item = usize>) {
        match self {
            Self::Fair(indexer) => indexer.dequeue_or_drain(range),
            Self::Shuffled(indexer) => indexer.drain(range),
            Self::Standard => {}
        }
    }

    pub(super) fn clear(&mut self) {
        match self {
            Self::Fair(indexer) => indexer.clear(),
            Self::Shuffled(indexer) => indexer.clear(),
            Self::Standard => {}
        }
    }
}

pub(super) struct FairIndexer {
    starting_index: usize,
    inner: Vec<(Id<UserMarker>, usize)>,
}

impl FairIndexer {
    pub(super) fn new<'a>(
        items: impl Iterator<Item = &'a super::queue::Item>,
        starting_index: usize,
    ) -> Self {
        let inner = items
            .skip(starting_index)
            .chunk_by(|c| c.requester)
            .into_iter()
            .map(|(r, g)| (r, g.count()))
            .collect();

        Self {
            starting_index,
            inner,
        }
    }

    fn iter_bucket_lens(&self) -> impl Iterator<Item = usize> + Clone + use<'_> {
        self.inner.iter().map(|(_, l)| l).copied()
    }

    fn iter_bucket_ranges(&self) -> impl Iterator<Item = std::ops::Range<usize>> + use<'_> {
        self.inner.iter().scan(self.starting_index, |i, (_, l)| {
            let j = *i;
            *i += l;
            Some(j..*i)
        })
    }

    fn iter_indices(&self) -> impl Iterator<Item = usize> + use<'_> {
        multi_interleave(chunked_range(self.starting_index, self.iter_bucket_lens()))
    }

    pub(super) fn current(&self, current_index: usize) -> Option<usize> {
        self.iter_indices().nth(current_index - self.starting_index)
    }

    pub(super) fn enqueue(&mut self, additional: usize, requester: Id<UserMarker>) {
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

pub(super) struct ShuffledIndexer(Vec<usize>);

impl ShuffledIndexer {
    pub(super) fn new(size: usize, starting_index: usize) -> Self {
        let mut rest = (0..size).collect::<Vec<_>>();
        let mut next = rest.drain(starting_index + 1..).collect::<Vec<_>>();
        next.shuffle(&mut rand::rng());
        rest.extend(next);

        Self(rest)
    }

    pub(super) fn current(&self, current_index: usize) -> Option<usize> {
        self.0.get(current_index).copied()
    }

    pub(super) fn enqueue(&mut self, additional: usize, current_index: usize) {
        let old_len = self.0.len();
        self.0.reserve(additional);

        let mut rng = rand::rng();
        (0..additional)
            .map(|d| rng.random_range(current_index + 1..=old_len + d))
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
