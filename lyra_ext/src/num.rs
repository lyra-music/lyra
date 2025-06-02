#[inline]
#[must_use]
#[expect(clippy::cast_possible_truncation)]
pub const fn u64_to_i64_truncating(n: u64) -> i64 {
    (n as i128) as i64
}

#[inline]
#[must_use]
#[expect(clippy::cast_possible_truncation)]
pub const fn usize_to_i64_truncating(n: usize) -> i64 {
    (n as i128) as i64
}

#[cfg(test)]
mod tests {
    use super::{u64_to_i64_truncating, usize_to_i64_truncating};
    use rstest::rstest;

    // Tests for u64_to_i64_truncating
    #[rstest]
    #[case::small_values(0u64, 0i64)]
    #[case::small_values(1u64, 1i64)]
    #[case::small_values(100u64, 100i64)]
    #[case::small_values(1000u64, 1000i64)]
    #[case::max_i64_fits(i64::MAX as u64, i64::MAX)]
    #[case::overflow_truncation(u64::MAX, -1i64)] // u64::MAX wraps to -1
    #[case::just_over_i64_max((i64::MAX as u64) + 1, i64::MIN)] // Wraps around
    #[case::mid_range_overflow(0x8000_0000_0000_0000u64, i64::MIN)]
    #[case::large_overflow(0xFFFF_FFFF_FFFF_FFFEu64, -2i64)]
    fn test_u64_to_i64_truncating(#[case] input: u64, #[case] expected: i64) {
        let result = u64_to_i64_truncating(input);
        assert_eq!(
            result, expected,
            "u64_to_i64_truncating({input}) should equal {expected}, got {result}"
        );
    }

    // Test boundary conditions for u64_to_i64_truncating
    #[test]
    fn test_u64_to_i64_truncating_boundaries() {
        // Test powers of 2 near the boundary
        let boundary = i64::MAX as u64;

        assert_eq!(u64_to_i64_truncating(boundary), i64::MAX);
        assert_eq!(u64_to_i64_truncating(boundary + 1), i64::MIN);
        assert_eq!(u64_to_i64_truncating(boundary + 2), i64::MIN + 1);
    }

    // Tests for usize_to_i64_truncating
    #[rstest]
    #[case::small_values(0usize, 0i64)]
    #[case::small_values(1usize, 1i64)]
    #[case::small_values(100usize, 100i64)]
    #[case::small_values(1000usize, 1000i64)]
    #[case::typical_size(
        usize::MAX.min(
            #[allow(clippy::cast_possible_truncation)]
            { i64::MAX as usize }
        ),
        {
            #[allow(clippy::cast_possible_wrap)]
            {
                (usize::MAX.min(
                    #[allow(clippy::cast_possible_truncation)]
                    { i64::MAX as usize }
                )) as i64
            }
        }
    )]
    fn test_usize_to_i64_truncating_safe_range(#[case] input: usize, #[case] expected: i64) {
        let result = usize_to_i64_truncating(input);
        assert_eq!(
            result, expected,
            "usize_to_i64_truncating({input}) should equal {expected}, got {result}"
        );
    }

    // Test usize edge cases (depends on platform)
    #[test]
    fn test_usize_to_i64_truncating_max() {
        let result = usize_to_i64_truncating(usize::MAX);

        // On 64-bit systems, this will truncate; on 32-bit systems, it should fit
        if std::mem::size_of::<usize>() == 8 {
            // 64-bit system: usize::MAX might overflow i64
            #[allow(clippy::cast_possible_truncation)]
            if usize::MAX > i64::MAX as usize {
                assert!(
                    result < 0,
                    "usize::MAX on 64-bit should truncate to negative i64"
                );
            } else {
                #[allow(clippy::cast_possible_wrap)]
                {
                    assert_eq!(result, usize::MAX as i64);
                }
            }
        } else {
            // 32-bit system: usize::MAX should always fit in i64
            #[allow(clippy::cast_possible_wrap)]
            {
                assert_eq!(result, usize::MAX as i64);
            }
            assert!(result >= 0, "usize::MAX on 32-bit should be positive i64");
        }
    }

    // Test const evaluation
    #[test]
    fn test_truncating_functions_const() {
        // These should be evaluable at compile time
        const U64_RESULT: i64 = u64_to_i64_truncating(12345u64);
        const USIZE_RESULT: i64 = usize_to_i64_truncating(67890usize);

        assert_eq!(U64_RESULT, 12345i64);
        assert_eq!(USIZE_RESULT, 67890i64);
    }

    // Test mathematical properties
    #[test]
    fn test_truncating_functions_properties() {
        // Test that small values are preserved
        for i in 0..1000u64 {
            #[allow(clippy::cast_possible_wrap)]
            {
                assert_eq!(
                    u64_to_i64_truncating(i),
                    i as i64,
                    "Small u64 value {i} should be preserved"
                );
            }
        }

        for i in 0..1000usize {
            #[allow(clippy::cast_possible_wrap)]
            {
                assert_eq!(
                    usize_to_i64_truncating(i),
                    i as i64,
                    "Small usize value {i} should be preserved"
                );
            }
        }
    }

    // Test performance (should be no-op at runtime due to const)
    #[rstest]
    #[case::performance_u64(1000)]
    #[case::performance_usize(1000)]
    fn test_truncating_functions_performance(#[case] iterations: usize) {
        let start = std::time::Instant::now();

        for i in 0..iterations {
            let _ = u64_to_i64_truncating(i as u64);
            let _ = usize_to_i64_truncating(i);
        }

        let duration = start.elapsed();

        // These should be extremely fast (essentially no-ops)
        assert!(
            duration < std::time::Duration::from_millis(10),
            "Truncating functions took too long: {duration:?} for {iterations} iterations"
        );
    }

    // Test specific overflow patterns
    #[rstest]
    #[case::overflow_patterns(0x8000_0000_0000_0000u64, i64::MIN)]
    #[case::overflow_patterns(0x8000_0000_0000_0001u64, i64::MIN + 1)]
    #[case::overflow_patterns(0xFFFF_FFFF_FFFF_FFFFu64, -1i64)]
    #[case::overflow_patterns(0xFFFF_FFFF_FFFF_FFFEu64, -2i64)]
    #[case::overflow_patterns(0xFFFF_FFFF_FFFF_FFFDu64, -3i64)]
    fn test_u64_to_i64_overflow_patterns(#[case] input: u64, #[case] expected: i64) {
        let result = u64_to_i64_truncating(input);
        assert_eq!(
            result, expected,
            "Overflow pattern: u64_to_i64_truncating(0x{input:016X}) should equal {expected}"
        );
    }

    // Test bit patterns are preserved in truncation
    #[rstest]
    fn test_truncating_bit_patterns() {
        // Test that the lower 64 bits are preserved exactly
        let test_values = [
            0x0123_4567_89AB_CDEFu64,
            0xFEDC_BA98_7654_3210u64,
            0xAAAA_AAAA_AAAA_AAAAu64,
            0x5555_5555_5555_5555u64,
        ];

        for &value in &test_values {
            let result = u64_to_i64_truncating(value);
            #[allow(clippy::cast_sign_loss)]
            let result_bits = result as u64;

            assert_eq!(
                result_bits, value,
                "Bit pattern should be preserved: 0x{value:016X} -> 0x{result_bits:016X}"
            );
        }
    }
}
