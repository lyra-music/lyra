use std::borrow::Cow;

use unicode_segmentation::UnicodeSegmentation;

pub trait AsGrapheme: UnicodeSegmentation {
    fn grapheme_len(&self) -> usize {
        self.graphemes(true).count()
    }

    fn grapheme_truncate(&self, new_len: usize) -> Cow<Self>
    where
        Self: ToOwned,
        Self::Owned: for<'a> FromIterator<&'a str>,
    {
        (self.grapheme_len() <= new_len)
            .then_some(Cow::Borrowed(self))
            .unwrap_or_else(|| Cow::Owned(self.graphemes(true).take(new_len).collect()))
    }
}

impl AsGrapheme for str {}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use crate::as_grapheme::AsGrapheme;

    #[rstest]
    #[case("", 0)]
    #[case("1", 1)]
    #[case("❤️‍🔥", 1)]
    #[case("❤️🔥", 2)]
    #[case("🇹🇭", 1)]
    #[case("🇹+🇭", 3)]
    #[case("🏳️‍⚧️ she/her", 9)]
    fn grapheme_len(#[case] input: &str, #[case] expected: usize) {
        assert_eq!(input.grapheme_len(), expected);
    }

    #[rstest]
    #[case("", "")]
    #[case("?", "")]
    #[case("🍄‍🟫", "")]
    #[case("🍄🟫", "")]
    #[case("🇬🇧", "")]
    #[case("🇬+🇧", "")]
    #[case("🙂‍↔️ Nope!", "")]
    fn grapheme_truncate_0(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(input.grapheme_truncate(0), expected);
    }

    #[rstest]
    #[case("", "")]
    #[case("!", "!")]
    #[case("🍋‍🟩", "🍋‍🟩")]
    #[case("🍋🟩", "🍋")]
    #[case("🇺🇸", "🇺🇸")]
    #[case("🇺+🇸", "🇺")]
    #[case("🙂‍↕️ Yep!", "🙂‍↕️")]
    fn grapheme_truncate_1(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(input.grapheme_truncate(1), expected);
    }

    #[rstest]
    #[case("", "")]
    #[case("#", "#")]
    #[case("🐦‍🔥", "🐦‍🔥")]
    #[case("🐦🔥", "🐦🔥")]
    #[case("🇯🇵", "🇯🇵")]
    #[case("🇯+🇵", "🇯+")]
    #[case("🧚🏻‍♀️I'm an angel!", "🧚🏻‍♀️I")]
    fn grapheme_truncate_2(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(input.grapheme_truncate(2), expected);
    }
}
