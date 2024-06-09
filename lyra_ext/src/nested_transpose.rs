pub trait NestedTranspose<T, A, B> {
    type Output: NestedTranspose<T, B, A>;

    fn transpose(self) -> Self::Output;
}

impl<T, E, F> NestedTranspose<T, E, F> for Result<Result<T, E>, F> {
    type Output = Result<Result<T, F>, E>;

    fn transpose(self) -> Self::Output {
        match self {
            Ok(Ok(t)) => Ok(Ok(t)),
            Ok(Err(e)) => Err(e),
            Err(f) => Ok(Err(f)),
        }
    }
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use super::NestedTranspose;

    #[derive(PartialEq, Debug)]
    struct T;
    #[derive(PartialEq, Debug)]
    struct E;
    #[derive(PartialEq, Debug)]
    struct F;

    #[rstest]
    #[case(Ok(Ok(T)), Ok(Ok(T)))]
    #[case(Ok(Err(E)), Err(E))]
    #[case(Err(F), Ok(Err(F)))]
    fn transpose(
        #[case] input: Result<Result<T, E>, F>,
        #[case] expected: Result<Result<T, F>, E>,
    ) {
        assert_eq!(input.transpose(), expected);
    }

    #[rstest]
    #[case(Ok(Ok(T)), Ok(Ok(T)))]
    #[case(Ok(Err(E)), Ok(Err(E)))]
    #[case(Err(F), Err(F))]
    fn transpose_transpose(
        #[case] input: Result<Result<T, E>, F>,
        #[case] expected: Result<Result<T, E>, F>,
    ) {
        assert_eq!(input.transpose().transpose(), expected);
    }
}
