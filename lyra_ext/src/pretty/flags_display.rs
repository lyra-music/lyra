use std::fmt::Display;

use bitflags::Flags;
use heck::ToTitleCase;

use crate::pretty::join::PrettyJoiner;

pub struct PrettyFlagsDisplayer<'a, T>
where
    T: Flags,
{
    inner: &'a T,
    code: bool,
}

impl<'a, T> Display for PrettyFlagsDisplayer<'a, T>
where
    T: Flags,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .inner
            .iter_names()
            .map(|(s, _)| {
                let flag = s.to_title_case();
                if self.code {
                    return format!("`{flag}`");
                }
                flag
            })
            .collect::<Box<_>>()
            .pretty_join_with_and();
        f.write_str(&s)
    }
}

pub trait PrettyFlagsDisplay: Flags {
    fn pretty_display(&self) -> PrettyFlagsDisplayer<Self> {
        PrettyFlagsDisplayer {
            inner: self,
            code: false,
        }
    }

    fn pretty_display_code(&self) -> PrettyFlagsDisplayer<Self> {
        PrettyFlagsDisplayer {
            inner: self,
            code: true,
        }
    }
}

#[cfg(test)]
mod test {
    use bitflags::bitflags;
    use rstest::rstest;

    use super::PrettyFlagsDisplay;

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

    impl PrettyFlagsDisplay for TestFlag {}

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
        assert_eq!(input.pretty_display().to_string(), expected);
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

    impl PrettyFlagsDisplay for TestFlag2 {}

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
        assert_eq!(input.pretty_display_code().to_string(), expected);
    }
}
