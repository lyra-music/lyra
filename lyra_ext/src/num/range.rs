use std::num::NonZeroUsize;

pub fn nonzero_usize_range_inclusive(
    start: impl Into<usize>,
    end: impl Into<usize>,
) -> impl Iterator<Item = NonZeroUsize> {
    (start.into()..=end.into()).filter_map(NonZeroUsize::new)
}

#[inline]
pub fn nonzero_usize_range_to_inclusive(
    end: impl Into<usize>,
) -> impl Iterator<Item = NonZeroUsize> {
    nonzero_usize_range_inclusive(NonZeroUsize::MIN, end)
}
