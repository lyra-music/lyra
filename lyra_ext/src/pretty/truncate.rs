use std::{borrow::Cow, ops::Add};

use crate::as_grapheme::AsGrapheme;

pub trait PrettyTruncator: AsGrapheme + ToOwned + 'static
where
    for<'a> Cow<'a, Self>: Add<&'a Self, Output = Cow<'a, Self>>,
{
    fn trail() -> &'static Self;
    fn pretty_truncate(&self, new_len: usize) -> Cow<Self>
    where
        for<'a> <Self as ToOwned>::Owned: FromIterator<&'a str>,
    {
        let trail = Self::trail();

        (self.grapheme_len() <= new_len)
            .then_some(Cow::Borrowed(self))
            .unwrap_or_else(|| self.grapheme_truncate(new_len - trail.grapheme_len()) + trail)
    }
}

impl PrettyTruncator for str {
    fn trail() -> &'static Self {
        "…"
    }
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use super::PrettyTruncator;

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
}
