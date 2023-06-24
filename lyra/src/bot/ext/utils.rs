use std::fmt::Debug;

use bitflags::Flags;
use convert_case::{Case, Casing};
use twilight_model::guild::Permissions;

pub trait EmptyStringMap: Sized {
    fn is_empty(&self) -> bool;

    fn or(self, other: impl Into<Self>) -> Self {
        if self.is_empty() {
            return other.into();
        }
        self
    }

    fn or_else(self, f: impl FnOnce() -> Self) -> Self {
        if self.is_empty() {
            return f();
        }
        self
    }
}

impl EmptyStringMap for String {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

trait PrettyJoiner: Sized {
    fn pretty_join(self, sep: impl Into<String>, ending_sep: impl Into<String>) -> String;

    fn pretty_join_with_and(self) -> String {
        self.pretty_join(", ", " and ")
    }

    fn pretty_join_with_or(self) -> String {
        self.pretty_join(", ", " or ")
    }
}

impl PrettyJoiner for &[String] {
    fn pretty_join(self, sep: impl Into<String>, ending_sep: impl Into<String>) -> String {
        match self {
            [] => String::new(),
            [first] => first.to_string(),
            [.., last] => {
                let joined = self[..self.len() - 1]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(&sep.into());
                format!("{}{}{}", joined, ending_sep.into(), last)
            }
        }
    }
}

pub trait BitFlagsPrettify: Debug {
    fn prettify(&self) -> String {
        format!("{:?}", self)
            .split(" | ")
            .map(|s| format!("`{}`", s.to_case(Case::Title)))
            .collect::<Vec<_>>()
            .pretty_join_with_and()
    }
}

impl BitFlagsPrettify for Permissions {}

// FIXME: Use this impl instead once twilight updated bitflags to 2.x.x
pub trait FlagsPrettify: Flags {
    fn prettify(&self) -> String {
        self.iter_names()
            .map(|(s, _)| s.to_case(Case::Title))
            .collect::<Vec<_>>()
            .pretty_join_with_and()
    }

    fn prettify_code(&self) -> String {
        self.iter_names()
            .map(|(s, _)| format!("`{}`", s.to_case(Case::Title)))
            .collect::<Vec<_>>()
            .pretty_join_with_and()
    }
}

#[cfg(test)]
mod tests {
    use bitflags::bitflags;
    use rstest::rstest;

    use crate::bot::ext::utils::{EmptyStringMap, PrettyJoiner};

    use super::FlagsPrettify;

    macro_rules! string_arr {
        ($($ty:literal),+) => {
            &[$($ty.to_string()),+]
        }
    }

    #[rstest]
    #[case("0", "0")]
    #[case("", "1")]
    fn test_or(#[case] input: String, #[case] expected: &str) {
        assert_eq!(input.or("1"), expected);
    }

    #[rstest]
    #[case("2", "2")]
    #[case("", "3")]
    fn test_or_else(#[case] input: String, #[case] expected: &str) {
        assert_eq!(input.or_else(|| "3".into()), expected);
    }

    #[rstest]
    #[case(&[], "")]
    #[case(string_arr!["0"], "0")]
    #[case(string_arr!["1", "2"], "1 > 2")]
    #[case(string_arr!["3", "4", "5"], "3 + 4 > 5")]
    #[case(string_arr!["6", "7", "8", "9"], "6 + 7 + 8 > 9")]
    fn test_pretty_join(#[case] input: &[String], #[case] expected: &str) {
        assert_eq!(input.pretty_join(" + ", " > "), expected);
    }

    #[rstest]
    #[case(&[], "")]
    #[case(string_arr!["a"], "a")]
    #[case(string_arr!["b", "c"], "b and c")]
    #[case(string_arr!["d", "e", "f"], "d, e and f")]
    #[case(string_arr!["g", "h", "i", "j"], "g, h, i and j")]
    fn test_pretty_join_with_and(#[case] input: &[String], #[case] expected: &str) {
        assert_eq!(input.pretty_join_with_and(), expected);
    }

    #[rstest]
    #[case(&[], "")]
    #[case(string_arr!["k"], "k")]
    #[case(string_arr!["l", "m"], "l or m")]
    #[case(string_arr!["n", "o", "p"], "n, o or p")]
    #[case(string_arr!["q", "r", "s", "t"], "q, r, s or t")]
    fn test_pretty_join_with_or(#[case] input: &[String], #[case] expected: &str) {
        assert_eq!(input.pretty_join_with_or(), expected);
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
    fn test_flags_prettify(#[case] input: TestFlag, #[case] expected: &str) {
        assert_eq!(input.prettify(), expected)
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
    fn test_flags_prettify_code(#[case] input: TestFlag2, #[case] expected: &str) {
        assert_eq!(input.prettify_code(), expected)
    }
}
