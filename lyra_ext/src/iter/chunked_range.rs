/* FIXME: make this generic over `T: std::ops::Add<Output = T> + std::ops::AddAssign + std::iter::Step + Copy` once `std::iter::Step` is stablised:
    https://github.com/rust-lang/rust/issues/42168
*/
pub fn chunked_range(
    start: usize,
    chunk_sizes: impl IntoIterator<Item = usize>,
) -> impl Iterator<Item = impl Iterator<Item = usize>> {
    let mut current_start = start;
    chunk_sizes.into_iter().map(move |chunk_size| {
        let range = current_start..current_start + chunk_size;
        current_start += chunk_size;
        range
    })
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use super::chunked_range;

    #[rstest]
    #[case(0, [], [])]
    #[case(1, [], [])]
    fn chunked_range_0(
        #[case] input_start: usize,
        #[case] input_chunk_sizes: [usize; 0],
        #[case] expected: [[usize; 0]; 0],
    ) {
        assert_eq!(
            chunked_range(input_start, input_chunk_sizes)
                .map(Vec::from_iter)
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[rstest]
    #[case(0, [0], [[       ]])]
    #[case(1, [0], [[       ]])]
    #[case(0, [1], [[0      ]])]
    #[case(1, [1], [[1      ]])]
    #[case(0, [2], [[0, 1   ]])]
    #[case(1, [2], [[1, 2   ]])]
    #[case(0, [3], [[0, 1, 2]])]
    #[case(1, [3], [[1, 2, 3]])]
    fn chunked_range_1<const M: usize>(
        #[case] input_start: usize,
        #[case] input_chunk_sizes: [usize; 1],
        #[case] expected: [[usize; M]; 1],
    ) {
        assert_eq!(
            chunked_range(input_start, input_chunk_sizes)
                .map(Vec::from_iter)
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[rstest]
    #[case(0, [0, 0], [vec![       ], vec![       ]])]
    #[case(1, [0, 0], [vec![       ], vec![       ]])]
    #[case(0, [0, 1], [vec![       ], vec![0      ]])]
    #[case(1, [0, 1], [vec![       ], vec![1      ]])]
    #[case(0, [0, 2], [vec![       ], vec![0, 1   ]])]
    #[case(1, [0, 2], [vec![       ], vec![1, 2   ]])]
    #[case(0, [0, 3], [vec![       ], vec![0, 1, 2]])]
    #[case(1, [0, 3], [vec![       ], vec![1, 2, 3]])]
    #[case(0, [1, 0], [vec![0      ], vec![       ]])]
    #[case(1, [1, 0], [vec![1      ], vec![       ]])]
    #[case(0, [1, 1], [vec![0      ], vec![1      ]])]
    #[case(1, [1, 1], [vec![1      ], vec![2      ]])]
    #[case(0, [1, 2], [vec![0      ], vec![1, 2   ]])]
    #[case(1, [1, 2], [vec![1      ], vec![2, 3   ]])]
    #[case(0, [1, 3], [vec![0      ], vec![1, 2, 3]])]
    #[case(1, [1, 3], [vec![1      ], vec![2, 3, 4]])]
    #[case(0, [2, 0], [vec![0, 1   ], vec![       ]])]
    #[case(1, [2, 0], [vec![1, 2   ], vec![       ]])]
    #[case(0, [2, 1], [vec![0, 1   ], vec![2      ]])]
    #[case(1, [2, 1], [vec![1, 2   ], vec![3      ]])]
    #[case(0, [2, 2], [vec![0, 1   ], vec![2, 3   ]])]
    #[case(1, [2, 2], [vec![1, 2   ], vec![3, 4   ]])]
    #[case(0, [2, 3], [vec![0, 1   ], vec![2, 3, 4]])]
    #[case(1, [2, 3], [vec![1, 2   ], vec![3, 4, 5]])]
    #[case(0, [3, 0], [vec![0, 1, 2], vec![       ]])]
    #[case(1, [3, 0], [vec![1, 2, 3], vec![       ]])]
    #[case(0, [3, 1], [vec![0, 1, 2], vec![3      ]])]
    #[case(1, [3, 1], [vec![1, 2, 3], vec![4      ]])]
    #[case(0, [3, 2], [vec![0, 1, 2], vec![3, 4   ]])]
    #[case(1, [3, 2], [vec![1, 2, 3], vec![4, 5   ]])]
    #[case(0, [3, 3], [vec![0, 1, 2], vec![3, 4, 5]])]
    #[case(1, [3, 3], [vec![1, 2, 3], vec![4, 5, 6]])]
    fn chunked_range_2(
        #[case] input_start: usize,
        #[case] input_chunk_sizes: [usize; 2],
        #[case] expected: [Vec<usize>; 2],
    ) {
        assert_eq!(
            chunked_range(input_start, input_chunk_sizes)
                .map(Vec::from_iter)
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[rstest]
    #[case(0, [0, 0, 0], [vec![       ], vec![       ], vec![       ]])]
    #[case(1, [0, 0, 0], [vec![       ], vec![       ], vec![       ]])]
    #[case(0, [0, 1, 0], [vec![       ], vec![0      ], vec![       ]])]
    #[case(1, [0, 1, 0], [vec![       ], vec![1      ], vec![       ]])]
    #[case(0, [0, 2, 0], [vec![       ], vec![0, 1   ], vec![       ]])]
    #[case(1, [0, 2, 0], [vec![       ], vec![1, 2   ], vec![       ]])]
    #[case(0, [0, 3, 0], [vec![       ], vec![0, 1, 2], vec![       ]])]
    #[case(1, [0, 3, 0], [vec![       ], vec![1, 2, 3], vec![       ]])]
    #[case(0, [1, 0, 0], [vec![0      ], vec![       ], vec![       ]])]
    #[case(1, [1, 0, 0], [vec![1      ], vec![       ], vec![       ]])]
    #[case(0, [1, 1, 0], [vec![0      ], vec![1      ], vec![       ]])]
    #[case(1, [1, 1, 0], [vec![1      ], vec![2      ], vec![       ]])]
    #[case(0, [1, 2, 0], [vec![0      ], vec![1, 2   ], vec![       ]])]
    #[case(1, [1, 2, 0], [vec![1      ], vec![2, 3   ], vec![       ]])]
    #[case(0, [1, 3, 0], [vec![0      ], vec![1, 2, 3], vec![       ]])]
    #[case(1, [1, 3, 0], [vec![1      ], vec![2, 3, 4], vec![       ]])]
    #[case(0, [2, 0, 0], [vec![0, 1   ], vec![       ], vec![       ]])]
    #[case(1, [2, 0, 0], [vec![1, 2   ], vec![       ], vec![       ]])]
    #[case(0, [2, 1, 0], [vec![0, 1   ], vec![2      ], vec![       ]])]
    #[case(1, [2, 1, 0], [vec![1, 2   ], vec![3      ], vec![       ]])]
    #[case(0, [2, 2, 0], [vec![0, 1   ], vec![2, 3   ], vec![       ]])]
    #[case(1, [2, 2, 0], [vec![1, 2   ], vec![3, 4   ], vec![       ]])]
    #[case(0, [2, 3, 0], [vec![0, 1   ], vec![2, 3, 4], vec![       ]])]
    #[case(1, [2, 3, 0], [vec![1, 2   ], vec![3, 4, 5], vec![       ]])]
    #[case(0, [3, 0, 0], [vec![0, 1, 2], vec![       ], vec![       ]])]
    #[case(1, [3, 0, 0], [vec![1, 2, 3], vec![       ], vec![       ]])]
    #[case(0, [3, 1, 0], [vec![0, 1, 2], vec![3      ], vec![       ]])]
    #[case(1, [3, 1, 0], [vec![1, 2, 3], vec![4      ], vec![       ]])]
    #[case(0, [3, 2, 0], [vec![0, 1, 2], vec![3, 4   ], vec![       ]])]
    #[case(1, [3, 2, 0], [vec![1, 2, 3], vec![4, 5   ], vec![       ]])]
    #[case(0, [3, 3, 0], [vec![0, 1, 2], vec![3, 4, 5], vec![       ]])]
    #[case(1, [3, 3, 0], [vec![1, 2, 3], vec![4, 5, 6], vec![       ]])]
    #[case(0, [0, 0, 1], [vec![       ], vec![       ], vec![0      ]])]
    #[case(1, [0, 0, 1], [vec![       ], vec![       ], vec![1      ]])]
    #[case(0, [0, 1, 1], [vec![       ], vec![0      ], vec![1      ]])]
    #[case(1, [0, 1, 1], [vec![       ], vec![1      ], vec![2      ]])]
    #[case(0, [0, 2, 1], [vec![       ], vec![0, 1   ], vec![2      ]])]
    #[case(1, [0, 2, 1], [vec![       ], vec![1, 2   ], vec![3      ]])]
    #[case(0, [0, 3, 1], [vec![       ], vec![0, 1, 2], vec![3      ]])]
    #[case(1, [0, 3, 1], [vec![       ], vec![1, 2, 3], vec![4      ]])]
    #[case(0, [1, 0, 1], [vec![0      ], vec![       ], vec![1      ]])]
    #[case(1, [1, 0, 1], [vec![1      ], vec![       ], vec![2      ]])]
    #[case(0, [1, 1, 1], [vec![0      ], vec![1      ], vec![2      ]])]
    #[case(1, [1, 1, 1], [vec![1      ], vec![2      ], vec![3      ]])]
    #[case(0, [1, 2, 1], [vec![0      ], vec![1, 2   ], vec![3      ]])]
    #[case(1, [1, 2, 1], [vec![1      ], vec![2, 3   ], vec![4      ]])]
    #[case(0, [1, 3, 1], [vec![0      ], vec![1, 2, 3], vec![4      ]])]
    #[case(1, [1, 3, 1], [vec![1      ], vec![2, 3, 4], vec![5      ]])]
    #[case(0, [2, 0, 1], [vec![0, 1   ], vec![       ], vec![2      ]])]
    #[case(1, [2, 0, 1], [vec![1, 2   ], vec![       ], vec![3      ]])]
    #[case(0, [2, 1, 1], [vec![0, 1   ], vec![2      ], vec![3      ]])]
    #[case(1, [2, 1, 1], [vec![1, 2   ], vec![3      ], vec![4      ]])]
    #[case(0, [2, 2, 1], [vec![0, 1   ], vec![2, 3   ], vec![4      ]])]
    #[case(1, [2, 2, 1], [vec![1, 2   ], vec![3, 4   ], vec![5      ]])]
    #[case(0, [2, 3, 1], [vec![0, 1   ], vec![2, 3, 4], vec![5      ]])]
    #[case(1, [2, 3, 1], [vec![1, 2   ], vec![3, 4, 5], vec![6      ]])]
    #[case(0, [3, 0, 1], [vec![0, 1, 2], vec![       ], vec![3      ]])]
    #[case(1, [3, 0, 1], [vec![1, 2, 3], vec![       ], vec![4      ]])]
    #[case(0, [3, 1, 1], [vec![0, 1, 2], vec![3      ], vec![4      ]])]
    #[case(1, [3, 1, 1], [vec![1, 2, 3], vec![4      ], vec![5      ]])]
    #[case(0, [3, 2, 1], [vec![0, 1, 2], vec![3, 4   ], vec![5      ]])]
    #[case(1, [3, 2, 1], [vec![1, 2, 3], vec![4, 5   ], vec![6      ]])]
    #[case(0, [3, 3, 1], [vec![0, 1, 2], vec![3, 4, 5], vec![6      ]])]
    #[case(1, [3, 3, 1], [vec![1, 2, 3], vec![4, 5, 6], vec![7      ]])]
    #[case(0, [0, 0, 2], [vec![       ], vec![       ], vec![0, 1   ]])]
    #[case(1, [0, 0, 2], [vec![       ], vec![       ], vec![1, 2   ]])]
    #[case(0, [0, 1, 2], [vec![       ], vec![0      ], vec![1, 2   ]])]
    #[case(1, [0, 1, 2], [vec![       ], vec![1      ], vec![2, 3   ]])]
    #[case(0, [0, 2, 2], [vec![       ], vec![0, 1   ], vec![2, 3   ]])]
    #[case(1, [0, 2, 2], [vec![       ], vec![1, 2   ], vec![3, 4   ]])]
    #[case(0, [0, 3, 2], [vec![       ], vec![0, 1, 2], vec![3, 4   ]])]
    #[case(1, [0, 3, 2], [vec![       ], vec![1, 2, 3], vec![4, 5   ]])]
    #[case(0, [1, 0, 2], [vec![0      ], vec![       ], vec![1, 2   ]])]
    #[case(1, [1, 0, 2], [vec![1      ], vec![       ], vec![2, 3   ]])]
    #[case(0, [1, 1, 2], [vec![0      ], vec![1      ], vec![2, 3   ]])]
    #[case(1, [1, 1, 2], [vec![1      ], vec![2      ], vec![3, 4   ]])]
    #[case(0, [1, 2, 2], [vec![0      ], vec![1, 2   ], vec![3, 4   ]])]
    #[case(1, [1, 2, 2], [vec![1      ], vec![2, 3   ], vec![4, 5   ]])]
    #[case(0, [1, 3, 2], [vec![0      ], vec![1, 2, 3], vec![4, 5   ]])]
    #[case(1, [1, 3, 2], [vec![1      ], vec![2, 3, 4], vec![5, 6   ]])]
    #[case(0, [2, 0, 2], [vec![0, 1   ], vec![       ], vec![2, 3   ]])]
    #[case(1, [2, 0, 2], [vec![1, 2   ], vec![       ], vec![3, 4   ]])]
    #[case(0, [2, 1, 2], [vec![0, 1   ], vec![2      ], vec![3, 4   ]])]
    #[case(1, [2, 1, 2], [vec![1, 2   ], vec![3      ], vec![4, 5   ]])]
    #[case(0, [2, 2, 2], [vec![0, 1   ], vec![2, 3   ], vec![4, 5   ]])]
    #[case(1, [2, 2, 2], [vec![1, 2   ], vec![3, 4   ], vec![5, 6   ]])]
    #[case(0, [2, 3, 2], [vec![0, 1   ], vec![2, 3, 4], vec![5, 6   ]])]
    #[case(1, [2, 3, 2], [vec![1, 2   ], vec![3, 4, 5], vec![6, 7   ]])]
    #[case(0, [3, 0, 2], [vec![0, 1, 2], vec![       ], vec![3, 4   ]])]
    #[case(1, [3, 0, 2], [vec![1, 2, 3], vec![       ], vec![4, 5   ]])]
    #[case(0, [3, 1, 2], [vec![0, 1, 2], vec![3      ], vec![4, 5   ]])]
    #[case(1, [3, 1, 2], [vec![1, 2, 3], vec![4      ], vec![5, 6   ]])]
    #[case(0, [3, 2, 2], [vec![0, 1, 2], vec![3, 4   ], vec![5, 6   ]])]
    #[case(1, [3, 2, 2], [vec![1, 2, 3], vec![4, 5   ], vec![6, 7   ]])]
    #[case(0, [3, 3, 2], [vec![0, 1, 2], vec![3, 4, 5], vec![6, 7   ]])]
    #[case(1, [3, 3, 2], [vec![1, 2, 3], vec![4, 5, 6], vec![7, 8   ]])]
    #[case(0, [0, 0, 3], [vec![       ], vec![       ], vec![0, 1, 2]])]
    #[case(1, [0, 0, 3], [vec![       ], vec![       ], vec![1, 2, 3]])]
    #[case(0, [0, 1, 3], [vec![       ], vec![0      ], vec![1, 2, 3]])]
    #[case(1, [0, 1, 3], [vec![       ], vec![1      ], vec![2, 3, 4]])]
    #[case(0, [0, 2, 3], [vec![       ], vec![0, 1   ], vec![2, 3, 4]])]
    #[case(1, [0, 2, 3], [vec![       ], vec![1, 2   ], vec![3, 4, 5]])]
    #[case(0, [0, 3, 3], [vec![       ], vec![0, 1, 2], vec![3, 4, 5]])]
    #[case(1, [0, 3, 3], [vec![       ], vec![1, 2, 3], vec![4, 5, 6]])]
    #[case(0, [1, 0, 3], [vec![0      ], vec![       ], vec![1, 2, 3]])]
    #[case(1, [1, 0, 3], [vec![1      ], vec![       ], vec![2, 3, 4]])]
    #[case(0, [1, 1, 3], [vec![0      ], vec![1      ], vec![2, 3, 4]])]
    #[case(1, [1, 1, 3], [vec![1      ], vec![2      ], vec![3, 4, 5]])]
    #[case(0, [1, 2, 3], [vec![0      ], vec![1, 2   ], vec![3, 4, 5]])]
    #[case(1, [1, 2, 3], [vec![1      ], vec![2, 3   ], vec![4, 5, 6]])]
    #[case(0, [1, 3, 3], [vec![0      ], vec![1, 2, 3], vec![4, 5, 6]])]
    #[case(1, [1, 3, 3], [vec![1      ], vec![2, 3, 4], vec![5, 6, 7]])]
    #[case(0, [2, 0, 3], [vec![0, 1   ], vec![       ], vec![2, 3, 4]])]
    #[case(1, [2, 0, 3], [vec![1, 2   ], vec![       ], vec![3, 4, 5]])]
    #[case(0, [2, 1, 3], [vec![0, 1   ], vec![2      ], vec![3, 4, 5]])]
    #[case(1, [2, 1, 3], [vec![1, 2   ], vec![3      ], vec![4, 5, 6]])]
    #[case(0, [2, 2, 3], [vec![0, 1   ], vec![2, 3   ], vec![4, 5, 6]])]
    #[case(1, [2, 2, 3], [vec![1, 2   ], vec![3, 4   ], vec![5, 6, 7]])]
    #[case(0, [2, 3, 3], [vec![0, 1   ], vec![2, 3, 4], vec![5, 6, 7]])]
    #[case(1, [2, 3, 3], [vec![1, 2   ], vec![3, 4, 5], vec![6, 7, 8]])]
    #[case(0, [3, 0, 3], [vec![0, 1, 2], vec![       ], vec![3, 4, 5]])]
    #[case(1, [3, 0, 3], [vec![1, 2, 3], vec![       ], vec![4, 5, 6]])]
    #[case(0, [3, 1, 3], [vec![0, 1, 2], vec![3      ], vec![4, 5, 6]])]
    #[case(1, [3, 1, 3], [vec![1, 2, 3], vec![4      ], vec![5, 6, 7]])]
    #[case(0, [3, 2, 3], [vec![0, 1, 2], vec![3, 4   ], vec![5, 6, 7]])]
    #[case(1, [3, 2, 3], [vec![1, 2, 3], vec![4, 5   ], vec![6, 7, 8]])]
    #[case(0, [3, 3, 3], [vec![0, 1, 2], vec![3, 4, 5], vec![6, 7, 8]])]
    #[case(1, [3, 3, 3], [vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]])]
    fn chunked_range_3(
        #[case] input_start: usize,
        #[case] input_chunk_sizes: [usize; 3],
        #[case] expected: [Vec<usize>; 3],
    ) {
        assert_eq!(
            chunked_range(input_start, input_chunk_sizes)
                .map(Vec::from_iter)
                .collect::<Vec<_>>(),
            expected
        );
    }
}
