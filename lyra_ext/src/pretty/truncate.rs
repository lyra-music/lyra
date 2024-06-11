use std::{borrow::Cow, ops::Add};

use crate::as_grapheme::AsGrapheme;

pub trait PrettyTruncator: AsGrapheme + ToOwned + 'static
where
    for<'a> Cow<'a, Self>: Add<&'a Self, Output = Cow<'a, Self>>,
{
    fn empty() -> &'static Self;
    fn trail() -> &'static Self;
    fn pretty_truncate(&self, new_len: usize) -> Cow<Self>
    where
        for<'a> <Self as ToOwned>::Owned: FromIterator<&'a str>,
    {
        let trail = Self::trail();

        (self.grapheme_len() <= new_len)
            .then_some(Cow::Borrowed(self))
            .unwrap_or_else(|| {
                let Some(len) = new_len.checked_sub(trail.grapheme_len()) else {
                    return Cow::Borrowed(Self::empty());
                };
                if len == 0 {
                    return Cow::Borrowed(trail);
                }
                self.grapheme_truncate(len) + trail
            })
    }
}

impl PrettyTruncator for str {
    fn empty() -> &'static Self {
        ""
    }

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
    #[case("1", "")]
    #[case("23", "")]
    #[case("456", "")]
    #[case("ตี", "")]
    #[case("งุงิ", "")]
    #[case("อิอิอิ", "")]
    #[case("❤️‍🩹", "")]
    #[case("👁️‍🗨️😶‍🌫️", "")]
    #[case("🏄‍♂️🐱‍🚀🏳️‍🌈", "")]
    #[case("Hello there!", "")]
    fn pretty_truncate_0(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(input.pretty_truncate(0), expected);
    }

    #[rstest]
    #[case("", "")]
    #[case("a", "a")]
    #[case("bc", "…")]
    #[case("def", "…")]
    #[case("ดี", "ดี")]
    #[case("มุมิ", "…")]
    #[case("จัตุรัส", "…")]
    #[case("🤹🏼‍♀️", "🤹🏼‍♀️")]
    #[case("👨🏻‍🚒🏳‍🟧‍⬛‍🟧", "…")]
    #[case("🐱‍👓👯🏾‍♂️🐦‍⬛", "…")]
    #[case("我想成为哥特女孩。。。", "…")]
    fn pretty_truncate_1(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(input.pretty_truncate(1), expected);
    }

    #[rstest]
    #[case("", "")]
    #[case("[", "[")]
    #[case("{}", "{}")]
    #[case("<=>", "<…")]
    #[case("🇳🇴", "🇳🇴")]
    #[case("🇿🇦🇲🇳", "🇿🇦🇲🇳")]
    #[case("🇸🇨🇷🇴🇱🇾", "🇸🇨…")]
    #[case("👨‍👶‍👦", "👨‍👶‍👦")]
    #[case("🏊🏼‍♂️🤼‍♀️", "🏊🏼‍♂️🤼‍♀️")]
    #[case("👩‍👩‍👧‍👦🤸🏼‍♂️😵‍💫", "👩‍👩‍👧‍👦…")]
    #[case("剣光よ、世の乱れを斬り尽くせ！", "剣…")]
    fn pretty_truncate_2(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(input.pretty_truncate(2), expected);
    }

    #[rstest]
    #[case("", "")]
    #[case(";", ";")]
    #[case("//", "//")]
    #[case("===", "===")]
    #[case("🇸🇴", "🇸🇴")]
    #[case("🇷🇴🇫🇮", "🇷🇴🇫🇮")]
    #[case("🇬🇬🇦🇱🇨🇨", "🇬🇬🇦🇱🇨🇨")]
    #[case("💆🏿‍♂️", "💆🏿‍♂️")]
    #[case("🤹‍♀️🐱‍🏍", "🤹‍♀️🐱‍🏍")]
    #[case("🏴‍☠️👮‍♂️🐻‍❄️", "🏴‍☠️👮‍♂️🐻‍❄️")]
    #[case("ลาลาลา ลาลา ลาลาลา ลา ลา ลา~", "ลา…")]
    fn pretty_truncate_3(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(input.pretty_truncate(3), expected);
    }
}
