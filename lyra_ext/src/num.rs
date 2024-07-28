#[inline]
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub const fn u64_to_i64_truncating(n: u64) -> i64 {
    (n as i128) as i64
}

#[inline]
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub const fn usize_to_i64_truncating(n: usize) -> i64 {
    (n as i128) as i64
}
