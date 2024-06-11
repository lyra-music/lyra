pub fn multi_interleave<A>(iters: A) -> MultiInterleave<<A::Item as IntoIterator>::IntoIter>
where
    A: IntoIterator,
    A::Item: IntoIterator,
    <A::Item as IntoIterator>::IntoIter: Iterator,
{
    MultiInterleave::new(iters.into_iter().map(IntoIterator::into_iter).collect())
}

pub struct MultiInterleave<I>
where
    I: Iterator,
{
    iterators: Box<[I]>,
    current: usize,
}

impl<I> MultiInterleave<I>
where
    I: Iterator,
{
    fn new(iterators: Box<[I]>) -> Self {
        Self {
            iterators,
            current: 0,
        }
    }
}

impl<I> Iterator for MultiInterleave<I>
where
    I: Iterator,
{
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::multi_interleave;

    #[rstest]
    #[case([], vec![])]
    fn multi_interleave_0(#[case] input: [Vec<u8>; 0], #[case] expected: Vec<u8>) {
        assert_eq!(multi_interleave(input).collect::<Vec<_>>(), expected);
    }

    #[rstest]
    #[case([vec![]]       , vec![])]
    #[case([vec![1]]      , vec![1])]
    #[case([vec![1, 2]]   , vec![1, 2])]
    #[case([vec![1, 2, 3]], vec![1, 2, 3])]
    fn multi_interleave_1(#[case] input: [Vec<u8>; 1], #[case] expected: Vec<u8>) {
        assert_eq!(multi_interleave(input).collect::<Vec<_>>(), expected);
    }

    #[rstest]
    #[case([vec![]       , vec![]]       , vec![])]
    #[case([vec![1]      , vec![]]       , vec![1])]
    #[case([vec![1, 2]   , vec![]]       , vec![1, 2])]
    #[case([vec![1, 2, 3], vec![]]       , vec![1, 2, 3])]
    #[case([vec![]       , vec![1]]      , vec![1])]
    #[case([vec![1]      , vec![1]]      , vec![1, 1])]
    #[case([vec![1, 2]   , vec![1]]      , vec![1, 1, 2])]
    #[case([vec![1, 2, 3], vec![1]]      , vec![1, 1, 2, 3])]
    #[case([vec![]       , vec![1, 2]]   , vec![1, 2])]
    #[case([vec![1]      , vec![1, 2]]   , vec![1, 1, 2])]
    #[case([vec![1, 2]   , vec![1, 2]]   , vec![1, 1, 2, 2])]
    #[case([vec![1, 2, 3], vec![1, 2]]   , vec![1, 1, 2, 2, 3])]
    #[case([vec![]       , vec![1, 2, 3]], vec![1, 2, 3])]
    #[case([vec![1]      , vec![1, 2, 3]], vec![1, 1, 2, 3])]
    #[case([vec![1, 2]   , vec![1, 2, 3]], vec![1, 1, 2, 2, 3])]
    #[case([vec![1, 2, 3], vec![1, 2, 3]], vec![1, 1, 2, 2, 3, 3])]
    fn multi_interleave_2(#[case] input: [Vec<u8>; 2], #[case] expected: Vec<u8>) {
        assert_eq!(multi_interleave(input).collect::<Vec<_>>(), expected);
    }

    #[rstest]
    #[case([vec![]       , vec![], vec![]], vec![])]
    #[case([vec![1]      , vec![], vec![]], vec![1])]
    #[case([vec![1, 2]   , vec![], vec![]], vec![1, 2])]
    #[case([vec![1, 2, 3], vec![], vec![]], vec![1, 2, 3])]
    #[case([vec![]       , vec![1], vec![]], vec![1])]
    #[case([vec![1]      , vec![1], vec![]], vec![1, 1])]
    #[case([vec![1, 2]   , vec![1], vec![]], vec![1, 1, 2])]
    #[case([vec![1, 2, 3], vec![1], vec![]], vec![1, 1, 2, 3])]
    #[case([vec![]       , vec![1, 2], vec![]], vec![1, 2])]
    #[case([vec![1]      , vec![1, 2], vec![]], vec![1, 1, 2])]
    #[case([vec![1, 2]   , vec![1, 2], vec![]], vec![1, 1, 2, 2])]
    #[case([vec![1, 2, 3], vec![1, 2], vec![]], vec![1, 1, 2, 2, 3])]
    #[case([vec![]       , vec![1, 2, 3], vec![]], vec![1, 2, 3])]
    #[case([vec![1]      , vec![1, 2, 3], vec![]], vec![1, 1, 2, 3])]
    #[case([vec![1, 2]   , vec![1, 2, 3], vec![]], vec![1, 1, 2, 2, 3])]
    #[case([vec![1, 2, 3], vec![1, 2, 3], vec![]], vec![1, 1, 2, 2, 3, 3])]
    #[case([vec![]       , vec![], vec![1]], vec![1])]
    #[case([vec![1]      , vec![], vec![1]], vec![1, 1])]
    #[case([vec![1, 2]   , vec![], vec![1]], vec![1, 1, 2])]
    #[case([vec![1, 2, 3], vec![], vec![1]], vec![1, 1, 2, 3])]
    #[case([vec![]       , vec![1], vec![1]], vec![1, 1])]
    #[case([vec![1]      , vec![1], vec![1]], vec![1, 1, 1])]
    #[case([vec![1, 2]   , vec![1], vec![1]], vec![1, 1, 1, 2])]
    #[case([vec![1, 2, 3], vec![1], vec![1]], vec![1, 1, 1, 2, 3])]
    #[case([vec![]       , vec![1, 2], vec![1]], vec![1, 1, 2])]
    #[case([vec![1]      , vec![1, 2], vec![1]], vec![1, 1, 1, 2])]
    #[case([vec![1, 2]   , vec![1, 2], vec![1]], vec![1, 1, 1, 2, 2])]
    #[case([vec![1, 2, 3], vec![1, 2], vec![1]], vec![1, 1, 1, 2, 2, 3])]
    #[case([vec![]       , vec![1, 2, 3], vec![1]], vec![1, 1, 2, 3])]
    #[case([vec![1]      , vec![1, 2, 3], vec![1]], vec![1, 1, 1, 2, 3])]
    #[case([vec![1, 2]   , vec![1, 2, 3], vec![1]], vec![1, 1, 1, 2, 2, 3])]
    #[case([vec![1, 2, 3], vec![1, 2, 3], vec![1]], vec![1, 1, 1, 2, 2, 3, 3])]
    #[case([vec![]       , vec![], vec![1, 2]], vec![1, 2])]
    #[case([vec![1]      , vec![], vec![1, 2]], vec![1, 1, 2])]
    #[case([vec![1, 2]   , vec![], vec![1, 2]], vec![1, 1, 2, 2])]
    #[case([vec![1, 2, 3], vec![], vec![1, 2]], vec![1, 1, 2, 2, 3])]
    #[case([vec![]       , vec![1], vec![1, 2]], vec![1, 1, 2])]
    #[case([vec![1]      , vec![1], vec![1, 2]], vec![1, 1, 1, 2])]
    #[case([vec![1, 2]   , vec![1], vec![1, 2]], vec![1, 1, 1, 2, 2])]
    #[case([vec![1, 2, 3], vec![1], vec![1, 2]], vec![1, 1, 1, 2, 2, 3])]
    #[case([vec![]       , vec![1, 2], vec![1, 2]], vec![1, 1, 2, 2])]
    #[case([vec![1]      , vec![1, 2], vec![1, 2]], vec![1, 1, 1, 2, 2])]
    #[case([vec![1, 2]   , vec![1, 2], vec![1, 2]], vec![1, 1, 1, 2, 2, 2])]
    #[case([vec![1, 2, 3], vec![1, 2], vec![1, 2]], vec![1, 1, 1, 2, 2, 2, 3])]
    #[case([vec![]       , vec![1, 2, 3], vec![1, 2]], vec![1, 1, 2, 2, 3])]
    #[case([vec![1]      , vec![1, 2, 3], vec![1, 2]], vec![1, 1, 1, 2, 2, 3])]
    #[case([vec![1, 2]   , vec![1, 2, 3], vec![1, 2]], vec![1, 1, 1, 2, 2, 2, 3])]
    #[case([vec![1, 2, 3], vec![1, 2, 3], vec![1, 2]], vec![1, 1, 1, 2, 2, 2, 3, 3])]
    #[case([vec![]       , vec![], vec![1, 2, 3]], vec![1, 2, 3])]
    #[case([vec![1]      , vec![], vec![1, 2, 3]], vec![1, 1, 2, 3])]
    #[case([vec![1, 2]   , vec![], vec![1, 2, 3]], vec![1, 1, 2, 2, 3])]
    #[case([vec![1, 2, 3], vec![], vec![1, 2, 3]], vec![1, 1, 2, 2, 3, 3])]
    #[case([vec![]       , vec![1], vec![1, 2, 3]], vec![1, 1, 2, 3])]
    #[case([vec![1]      , vec![1], vec![1, 2, 3]], vec![1, 1, 1, 2, 3])]
    #[case([vec![1, 2]   , vec![1], vec![1, 2, 3]], vec![1, 1, 1, 2, 2, 3])]
    #[case([vec![1, 2, 3], vec![1], vec![1, 2, 3]], vec![1, 1, 1, 2, 2, 3, 3])]
    #[case([vec![]       , vec![1, 2], vec![1, 2, 3]], vec![1, 1, 2, 2, 3])]
    #[case([vec![1]      , vec![1, 2], vec![1, 2, 3]], vec![1, 1, 1, 2, 2, 3])]
    #[case([vec![1, 2]   , vec![1, 2], vec![1, 2, 3]], vec![1, 1, 1, 2, 2, 2, 3])]
    #[case([vec![1, 2, 3], vec![1, 2], vec![1, 2, 3]], vec![1, 1, 1, 2, 2, 2, 3, 3])]
    #[case([vec![]       , vec![1, 2, 3], vec![1, 2, 3]], vec![1, 1, 2, 2, 3, 3])]
    #[case([vec![1]      , vec![1, 2, 3], vec![1, 2, 3]], vec![1, 1, 1, 2, 2, 3, 3])]
    #[case([vec![1, 2]   , vec![1, 2, 3], vec![1, 2, 3]], vec![1, 1, 1, 2, 2, 2, 3, 3])]
    #[case([vec![1, 2, 3], vec![1, 2, 3], vec![1, 2, 3]], vec![1, 1, 1, 2, 2, 2, 3, 3, 3])]
    fn multi_interleave_3(#[case] input: [Vec<u8>; 3], #[case] expected: Vec<u8>) {
        assert_eq!(multi_interleave(input).collect::<Vec<_>>(), expected);
    }
}
