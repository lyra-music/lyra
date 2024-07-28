use std::borrow::Borrow;

pub trait Join<J> {
    type Joined;

    fn pretty_join(slice: &Self, sep: J, last_sep: J) -> Self::Joined;
}

pub trait PrettyJoiner {
    type Joiner;

    fn sep() -> Self::Joiner;
    fn and() -> Self::Joiner;
    fn or() -> Self::Joiner;

    fn pretty_join<J>(&self, sep: J, last_sep: J) -> <Self as Join<J>>::Joined
    where
        Self: Join<J>,
    {
        Join::pretty_join(self, sep, last_sep)
    }
    fn pretty_join_with(&self, last_sep: Self::Joiner) -> <Self as Join<Self::Joiner>>::Joined
    where
        Self: Join<Self::Joiner>,
    {
        Join::pretty_join(self, Self::sep(), last_sep)
    }
    fn pretty_join_with_and(&self) -> <Self as Join<Self::Joiner>>::Joined
    where
        Self: Join<Self::Joiner>,
    {
        Join::pretty_join(self, Self::sep(), Self::and())
    }
    fn pretty_join_with_or(&self) -> <Self as Join<Self::Joiner>>::Joined
    where
        Self: Join<Self::Joiner>,
    {
        Join::pretty_join(self, Self::sep(), Self::or())
    }
}

impl<S: Borrow<str>> Join<&str> for [S] {
    type Joined = String;

    fn pretty_join(slice: &Self, sep: &str, last_sep: &str) -> Self::Joined {
        match slice {
            [] => String::new(),
            [first] => first.borrow().to_owned(),
            [.., last] => {
                let joined = slice[..slice.len() - 1]
                    .iter()
                    .map(|s| s.borrow().to_owned())
                    .collect::<Vec<_>>()
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

#[cfg(test)]
mod test {
    use rstest::rstest;

    use super::PrettyJoiner;

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
}
