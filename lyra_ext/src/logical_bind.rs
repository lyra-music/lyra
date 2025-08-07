use std::borrow::Cow;

pub trait LogicalBind {
    fn is_truthy(&self) -> bool;

    fn or(&'_ self, other: impl Into<<Self as ToOwned>::Owned>) -> Cow<'_, Self>
    where
        Self: ToOwned,
    {
        if self.is_truthy() {
            return Cow::Borrowed(self);
        }
        Cow::Owned(other.into())
    }

    fn or_else(&'_ self, f: impl FnOnce() -> <Self as ToOwned>::Owned) -> Cow<'_, Self>
    where
        Self: ToOwned,
    {
        if self.is_truthy() {
            return Cow::Borrowed(self);
        }
        Cow::Owned(f())
    }
}

impl LogicalBind for str {
    fn is_truthy(&self) -> bool {
        !self.is_empty()
    }
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use crate::logical_bind::LogicalBind;

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
}
