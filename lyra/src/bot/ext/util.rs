use std::{
    borrow::{Borrow, Cow},
    fmt::{Debug, Display},
    str::FromStr,
};

use bitflags::Flags;
use chrono::Duration;
use heck::ToTitleCase;
use itertools::Itertools;
use twilight_model::guild::Permissions;
use unicode_segmentation::UnicodeSegmentation;

use crate::bot::{
    core::r#const::regex, error::PrettifiedTimestampParse as PrettifiedTimestampParseError,
};

pub trait OptionMap {
    fn is_none(&self) -> bool;

    fn or(&self, other: impl Into<<Self as ToOwned>::Owned>) -> Cow<Self>
    where
        Self: ToOwned,
    {
        if self.is_none() {
            return Cow::Owned(other.into());
        }
        Cow::Borrowed(self)
    }

    fn or_else(&self, f: impl FnOnce() -> <Self as ToOwned>::Owned) -> Cow<Self>
    where
        Self: ToOwned,
    {
        if self.is_none() {
            return Cow::Owned(f());
        }
        Cow::Borrowed(self)
    }
}

impl OptionMap for str {
    fn is_none(&self) -> bool {
        self.is_empty()
    }
}

pub trait PrettyJoin<J> {
    type Joined;

    fn pretty_join(slice: &Self, sep: J, last_sep: J) -> Self::Joined;
}

pub trait PrettyJoiner {
    type Joiner;

    fn sep() -> Self::Joiner;
    fn and() -> Self::Joiner;
    fn or() -> Self::Joiner;

    fn pretty_join<J>(&self, sep: J, last_sep: J) -> <Self as PrettyJoin<J>>::Joined
    where
        Self: PrettyJoin<J>,
    {
        PrettyJoin::pretty_join(self, sep, last_sep)
    }
    fn pretty_join_with(&self, last_sep: Self::Joiner) -> <Self as PrettyJoin<Self::Joiner>>::Joined
    where
        Self: PrettyJoin<Self::Joiner>,
    {
        PrettyJoin::pretty_join(self, Self::sep(), last_sep)
    }
    fn pretty_join_with_and(&self) -> <Self as PrettyJoin<Self::Joiner>>::Joined
    where
        Self: PrettyJoin<Self::Joiner>,
    {
        PrettyJoin::pretty_join(self, Self::sep(), Self::and())
    }
    fn pretty_join_with_or(&self) -> <Self as PrettyJoin<Self::Joiner>>::Joined
    where
        Self: PrettyJoin<Self::Joiner>,
    {
        PrettyJoin::pretty_join(self, Self::sep(), Self::or())
    }
}

impl<S: Borrow<str>> PrettyJoin<&str> for [S] {
    type Joined = String;

    fn pretty_join(slice: &Self, sep: &str, last_sep: &str) -> Self::Joined {
        match slice {
            [] => String::new(),
            [first] => first.borrow().to_owned(),
            [.., last] => {
                let joined = slice[..slice.len() - 1]
                    .iter()
                    .map(|s| s.borrow().to_owned())
                    .join(sep);
                joined + last_sep + last.borrow()
            }
        }
    }
}

impl<S: Borrow<str>> PrettyJoiner for [S] {
    type Joiner = &'static str;

    fn sep() -> Self::Joiner {
        ", "
    }
    fn and() -> Self::Joiner {
        " and "
    }
    fn or() -> Self::Joiner {
        " or "
    }
}

pub trait ViaGrapheme: UnicodeSegmentation {
    fn grapheme_len(&self) -> usize {
        self.graphemes(true).count()
    }

    fn grapheme_truncate(&self, new_len: usize) -> Cow<Self>
    where
        Self: ToOwned,
        <Self as ToOwned>::Owned: for<'a> FromIterator<&'a str>,
    {
        if self.grapheme_len() <= new_len {
            return Cow::Borrowed(self);
        }
        Cow::Owned(self.graphemes(true).take(new_len).collect())
    }
}

impl ViaGrapheme for str {}

pub trait PrettyTruncator: ViaGrapheme {
    fn trail() -> &'static Self;
    fn pretty_truncate(&self, new_len: usize) -> Cow<Self>
    where
        Self: ToOwned;
}

impl PrettyTruncator for str {
    fn trail() -> &'static Self {
        "…"
    }

    fn pretty_truncate(&self, new_len: usize) -> Cow<Self>
    where
        Self: ToOwned,
    {
        let trail = Self::trail();
        if self.grapheme_len() <= new_len {
            return Cow::Borrowed(self);
        }
        let truncated = self.grapheme_truncate(new_len - trail.grapheme_len());
        truncated + trail
    }
}

pub trait BitFlagsPrettify: Debug {
    fn prettify_code(&self) -> String {
        format!("{self:?}")
            .split(" | ")
            .map(|s| format!("`{}`", s.to_title_case()))
            .collect::<Vec<_>>()
            .pretty_join_with_and()
    }
}

impl BitFlagsPrettify for Permissions {}

// FIXME: Use this impl instead once twilight updated bitflags to 2.x.x
pub trait FlagsPrettify: Flags {
    fn prettify(&self) -> String {
        self.iter_names()
            .map(|(s, _)| s.to_title_case())
            .collect::<Vec<_>>()
            .pretty_join_with_and()
    }

    fn prettify_code(&self) -> String {
        self.iter_names()
            .map(|(s, _)| format!("`{}`", s.to_title_case()))
            .collect::<Vec<_>>()
            .pretty_join_with_and()
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct PrettifiedTimestamp(Duration);

impl std::ops::Deref for PrettifiedTimestamp {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for PrettifiedTimestamp {
    type Err = PrettifiedTimestampParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let captures = if let Some(captures) = regex::TIMESTAMP.captures(value) {
            captures
        } else if let Some(captures) = regex::TIMESTAMP_2.captures(value) {
            captures
        } else {
            return Err(PrettifiedTimestampParseError);
        };

        let ms = captures
            .name("ms")
            .and_then(|c| c.as_str().parse().ok())
            .unwrap_or(0);
        let s = captures
            .name("s")
            .and_then(|c| c.as_str().parse().ok())
            .unwrap_or(0);
        let m = captures
            .name("m")
            .or_else(|| captures.name("m1"))
            .or_else(|| captures.name("m2"))
            .and_then(|c| c.as_str().parse().ok())
            .unwrap_or(0);
        let h = captures
            .name("h")
            .and_then(|c| c.as_str().parse().ok())
            .unwrap_or(0);

        let total_ms = (((h * 60 + m) * 60 + s) * 1000) + ms;
        Ok(Self(Duration::milliseconds(total_ms)))
    }
}

impl Display for PrettifiedTimestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let divrem = |x, y| (x / y, x % y);

        let (s, ms) = divrem(self.0.num_milliseconds(), 1000);
        let (m, s) = divrem(s, 60);
        let (h, m) = divrem(m, 60);

        match (h, m, s) {
            (0, 0, 0) => write!(f, "0:00.{ms:03}"),
            (0, m, s) => write!(f, "{m}:{s:02}"),
            (h, m, s) => write!(f, "{h}:{m:02}:{s:02}"),
        }
    }
}

impl From<Duration> for PrettifiedTimestamp {
    fn from(value: Duration) -> Self {
        Self(value)
    }
}

impl From<PrettifiedTimestamp> for Duration {
    fn from(PrettifiedTimestamp(value): PrettifiedTimestamp) -> Self {
        value
    }
}

pub fn multi_interleave<I: Iterator>(iterators: Vec<I>) -> MultiInterleave<I> {
    MultiInterleave::new(iterators)
}

pub struct MultiInterleave<I: Iterator> {
    iterators: Vec<I>,
    current: usize,
}

impl<I: Iterator> MultiInterleave<I> {
    fn new(iterators: Vec<I>) -> Self {
        Self {
            iterators,
            current: 0,
        }
    }
}

impl<I: Iterator> Iterator for MultiInterleave<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let iterators_len = self.iterators.len();
        if iterators_len == 0 {
            return None;
        }

        let mut exhausted = 0;
        while exhausted < iterators_len {
            let current_iter = &mut self.iterators[self.current];
            self.current = (self.current + 1) % iterators_len;
            if let Some(item) = current_iter.next() {
                return Some(item);
            }
            exhausted += 1;
        }

        None
    }
}

pub fn chunked_range(start: usize, chunk_sizes: Vec<usize>) -> impl Iterator<Item = Vec<usize>> {
    let end = start + chunk_sizes.iter().sum::<usize>();
    let mut range = start..end;
    chunk_sizes
        .into_iter()
        .map(move |d| range.by_ref().take(d).collect())
}

pub trait NestedTranspose<T, E, F> {
    fn transpose(self) -> impl NestedTranspose<T, F, E>;
}

impl<T, E, F> NestedTranspose<T, E, F> for Result<Result<T, E>, F> {
    fn transpose(self) -> Result<Result<T, F>, E> {
        match self {
            Ok(Ok(t)) => Ok(Ok(t)),
            Ok(Err(e)) => Err(e),
            Err(f) => Ok(Err(f)),
        }
    }
}

pub const fn rgb_to_hex(rgb: [u8; 3]) -> u32 {
    ((rgb[0] as u32) << 16) | ((rgb[1] as u32) << 8) | (rgb[2] as u32)
}

pub const fn hex_to_rgb(hex: u32) -> [u8; 3] {
    [
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bitflags::bitflags;
    use chrono::Duration;
    use rstest::rstest;

    use super::{FlagsPrettify, PrettifiedTimestamp};
    use crate::bot::{
        error::PrettifiedTimestampParse,
        ext::util::{OptionMap, PrettyJoiner, PrettyTruncator},
    };

    #[rstest]
    #[case("0", "0")]
    #[case("", "1")]
    fn string_or(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(input.or("1"), expected);
    }

    #[rstest]
    #[case("2", "2")]
    #[case("", "3")]
    fn string_or_else(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(input.or_else(|| "3".into()), expected);
    }

    #[rstest]
    #[case([], "")]
    #[case(["0"], "0")]
    #[case(["1", "2"], "1 > 2")]
    #[case(["3", "4", "5"], "3 + 4 > 5")]
    #[case(["6", "7", "8", "9"], "6 + 7 + 8 > 9")]
    fn string_pretty_join<const N: usize>(#[case] input: [&str; N], #[case] expected: &str) {
        assert_eq!(input.pretty_join(" + ", " > "), expected);
    }

    #[rstest]
    #[case([], "")]
    #[case(["a"], "a")]
    #[case(["b", "c"], "b and c")]
    #[case(["d", "e", "f"], "d, e and f")]
    #[case(["g", "h", "i", "j"], "g, h, i and j")]
    fn string_pretty_join_with_and<const N: usize>(
        #[case] input: [&str; N],
        #[case] expected: &str,
    ) {
        assert_eq!(input.pretty_join_with_and(), expected);
    }

    #[rstest]
    #[case([], "")]
    #[case(["k"], "k")]
    #[case(["l", "m"], "l or m")]
    #[case(["n", "o", "p"], "n, o or p")]
    #[case(["q", "r", "s", "t"], "q, r, s or t")]
    fn string_pretty_join_with_or<const N: usize>(
        #[case] input: [&str; N],
        #[case] expected: &str,
    ) {
        assert_eq!(input.pretty_join_with_or(), expected);
    }

    #[rstest]
    #[case("", "")]
    #[case("1", "1")]
    #[case("234", "234")]
    #[case("5678", "56…")]
    #[case("竪琴を弾く", "竪琴…")]
    #[case("การเขียนโปรแกรม", "กา…")]
    #[case("😶‍🌫️😮‍💨😵‍💫❤️‍🔥❤️‍🩹👁️‍🗨️", "😶‍🌫️😮‍💨…")]
    fn string_pretty_truncate(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(input.pretty_truncate(3), expected);
    }

    bitflags! {
        struct TestFlag: u8 {
            const ONE = 0b001;
            const ANOTHER_ONE = 0b010;
            const EVEN_ANOTHER_ONE = 0b100;

            const ONE_AND_ANOTHER_ONE = Self::ONE.bits() | Self::ANOTHER_ONE.bits();
            const ANOTHER_ONE_AND_EVEN_ANOTHER_ONE = Self::ANOTHER_ONE.bits() | Self::EVEN_ANOTHER_ONE.bits();
            const ONE_AND_EVEN_ANOTHER_ONE = Self::ONE.bits() | Self::EVEN_ANOTHER_ONE.bits();

            const ALL = Self::ONE.bits() | Self::ANOTHER_ONE.bits() | Self::EVEN_ANOTHER_ONE.bits();
        }
    }

    impl FlagsPrettify for TestFlag {}

    #[rstest]
    #[case(TestFlag::empty(), "")]
    #[case(TestFlag::ONE, "One")]
    #[case(TestFlag::ANOTHER_ONE, "Another One")]
    #[case(TestFlag::EVEN_ANOTHER_ONE, "Even Another One")]
    #[case(TestFlag::ONE_AND_ANOTHER_ONE, "One and Another One")]
    #[case(
        TestFlag::ANOTHER_ONE_AND_EVEN_ANOTHER_ONE,
        "Another One and Even Another One"
    )]
    #[case(TestFlag::ONE_AND_EVEN_ANOTHER_ONE, "One and Even Another One")]
    #[case(TestFlag::ALL, "One, Another One and Even Another One")]
    fn flags_prettify(#[case] input: TestFlag, #[case] expected: &str) {
        assert_eq!(input.prettify(), expected);
    }

    bitflags! {
        struct TestFlag2: u8 {
            const TWO = 0b001;
            const OTHER_TWO = 0b010;
            const OTHER_TWO_ELSE = 0b100;

            const TWO_AND_OTHER_TWO = Self::TWO.bits() | Self::OTHER_TWO.bits();
            const OTHER_TWO_AND_OTHER_TWO_ELSE = Self::OTHER_TWO.bits() | Self::OTHER_TWO_ELSE.bits();
            const TWO_AND_OTHER_TWO_ELSE = Self::TWO.bits() | Self::OTHER_TWO_ELSE.bits();

            const ALL = Self::TWO.bits() | Self::OTHER_TWO.bits() | Self::OTHER_TWO_ELSE.bits();
        }
    }

    impl FlagsPrettify for TestFlag2 {}

    #[rstest]
    #[case(TestFlag2::empty(), "")]
    #[case(TestFlag2::TWO, "`Two`")]
    #[case(TestFlag2::OTHER_TWO, "`Other Two`")]
    #[case(TestFlag2::OTHER_TWO_ELSE, "`Other Two Else`")]
    #[case(TestFlag2::TWO_AND_OTHER_TWO, "`Two` and `Other Two`")]
    #[case(
        TestFlag2::OTHER_TWO_AND_OTHER_TWO_ELSE,
        "`Other Two` and `Other Two Else`"
    )]
    #[case(TestFlag2::TWO_AND_OTHER_TWO_ELSE, "`Two` and `Other Two Else`")]
    #[case(TestFlag2::ALL, "`Two`, `Other Two` and `Other Two Else`")]
    fn flags_prettify_code(#[case] input: TestFlag2, #[case] expected: &str) {
        assert_eq!(input.prettify_code(), expected);
    }

    #[rstest]
    #[case(Duration::zero().into(), "0:00.000")]
    #[case(Duration::milliseconds(999).into(), "0:00.999")]
    #[case(Duration::seconds(1).into(), "0:01")]
    #[case(Duration::seconds(59).into(), "0:59")]
    #[case(Duration::minutes(1).into(), "1:00")]
    #[case(Duration::seconds(61).into(), "1:01")]
    #[case(Duration::seconds(59*60 + 59).into(),"59:59")]
    #[case(Duration::hours(1).into(), "1:00:00")]
    #[case(Duration::seconds(60*60 + 1).into(), "1:00:01")]
    #[case(Duration::seconds(60*60 + 59).into(),"1:00:59")]
    #[case(Duration::minutes(61).into(), "1:01:00")]
    #[case(Duration::seconds(60*60 + 61).into(), "1:01:01")]
    #[case(Duration::seconds(999*60*60 + 59*60 + 59).into(), "999:59:59")]
    fn prettified_timestamp_to_string(#[case] input: PrettifiedTimestamp, #[case] expected: &str) {
        assert_eq!(input.to_string(), expected);
    }

    #[rstest]
    #[case("0:0", Err(PrettifiedTimestampParse))]
    #[case("0:00", Ok(Duration::zero().into()))]
    #[case("0:00.0", Err(PrettifiedTimestampParse))]
    #[case("0:00.999", Ok(Duration::milliseconds(999).into()))]
    #[case("0:00.9999", Err(PrettifiedTimestampParse))]
    #[case("0:01", Ok(Duration::seconds(1).into()))]
    #[case("0:59.999", Ok(Duration::milliseconds(59_999).into()))]
    #[case("0:99.999", Err(PrettifiedTimestampParse))]
    #[case("1:00", Ok(Duration::minutes(1).into()))]
    #[case("1:00.999", Ok(Duration::milliseconds(60_999).into()))]
    #[case("1:01", Ok(Duration::seconds(61).into()))]
    #[case("59:59.999", Ok(Duration::milliseconds(59*60_000 + 59_999).into()))]
    #[case("99:59.999", Err(PrettifiedTimestampParse))]
    #[case("0:0:00", Err(PrettifiedTimestampParse))]
    #[case("0:00:00", Err(PrettifiedTimestampParse))]
    #[case("1:00:00", Ok(Duration::hours(1).into()))]
    #[case("1:00:00.999", Ok(Duration::milliseconds(60*60_000 + 999).into()))]
    #[case("1:00:01", Ok(Duration::seconds(60*60 + 1).into()))]
    #[case("1:00:59.999", Ok(Duration::milliseconds(60*60_000 + 59_999).into()))]
    #[case("1:01:00", Ok(Duration::minutes(61).into()))]
    #[case("1:01:00.999", Ok(Duration::milliseconds(60*60_000 + 60_999).into()))]
    #[case("1:01:01", Ok(Duration::seconds(60*60 + 61).into()))]
    #[case("999:59:59.999", Ok(Duration::milliseconds(999*60*60_000 + 59*60_000 + 59_999).into()))]
    #[case("", Ok(Duration::zero().into()))]
    #[case("0ms", Err(PrettifiedTimestampParse))]
    #[case("01ms", Err(PrettifiedTimestampParse))]
    #[case("999ms", Ok(Duration::milliseconds(999).into()))]
    #[case("9999ms", Err(PrettifiedTimestampParse))]
    #[case("0s", Err(PrettifiedTimestampParse))]
    #[case("1s", Ok(Duration::seconds(1).into()))]
    #[case("01s", Err(PrettifiedTimestampParse))]
    #[case("59 sec 999 msec", Ok(Duration::milliseconds(59_999).into()))]
    #[case("99s999ms", Err(PrettifiedTimestampParse))]
    #[case("0m", Err(PrettifiedTimestampParse))]
    #[case("1m", Ok(Duration::minutes(1).into()))]
    #[case("01m", Err(PrettifiedTimestampParse))]
    #[case("1m 999ms", Ok(Duration::milliseconds(60_999).into()))]
    #[case("1m1s", Ok(Duration::seconds(61).into()))]
    #[case("59 min 59 sec 999 msec", Ok(Duration::milliseconds(59*60_000 + 59_999).into()))]
    #[case("99m59s999ms", Err(PrettifiedTimestampParse))]
    #[case("0h", Err(PrettifiedTimestampParse))]
    #[case("1h", Ok(Duration::hours(1).into()))]
    #[case("01h", Err(PrettifiedTimestampParse))]
    #[case("1h 999ms", Ok(Duration::milliseconds(60*60_000 + 999).into()))]
    #[case("1h 1s", Ok(Duration::seconds(60*60 + 1).into()))]
    #[case("1h 59s 999ms", Ok(Duration::milliseconds(60*60_000 + 59_999).into()))]
    #[case("1h1m", Ok(Duration::minutes(61).into()))]
    #[case("1h1m 999ms", Ok(Duration::milliseconds(60*60_000 + 60_999).into()))]
    #[case("1h1m1s", Ok(Duration::seconds(60*60 + 61).into()))]
    #[case("999 hr 59 min 59 sec 999 msec", Ok(Duration::milliseconds(999*60*60_000 + 59*60_000 + 59_999).into()))]
    fn prettified_timestamp_from_str(
        #[case] input: &str,
        #[case] expected: Result<PrettifiedTimestamp, PrettifiedTimestampParse>,
    ) {
        assert_eq!(PrettifiedTimestamp::from_str(input), expected);
    }
}
