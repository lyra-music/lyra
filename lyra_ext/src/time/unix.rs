use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// # Panics
/// if system clock went backwards
#[must_use]
pub fn unix() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must move forward")
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::unix;
    use rstest::rstest;

    // Test that unix() returns a valid Duration
    #[test]
    fn test_unix_returns_valid_duration() {
        let result = unix();

        // Should be a reasonable timestamp (after year 2000, before year 2100)
        let year_2000_seconds = 946_684_800_u64; // 2000-01-01 00:00:00 UTC
        let year_2100_seconds = 4_102_444_800_u64; // 2100-01-01 00:00:00 UTC

        assert!(
            result.as_secs() >= year_2000_seconds,
            "Unix timestamp {result:?} should be after year 2000"
        );
        assert!(
            result.as_secs() < year_2100_seconds,
            "Unix timestamp {result:?} should be before year 2100"
        );
    }

    // Test that consecutive calls return increasing timestamps
    #[test]
    fn test_unix_chronological_order() {
        let first = unix();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let second = unix();

        assert!(
            second >= first,
            "Second timestamp {second:?} should be >= first timestamp {first:?}"
        );
    }

    // Test precision and consistency across multiple calls
    #[rstest]
    #[case::precision_consistency(5)]
    #[case::precision_consistency(10)]
    #[case::precision_consistency(25)]
    fn test_unix_precision_consistency(#[case] num_calls: usize) {
        let mut timestamps = Vec::with_capacity(num_calls);

        for _ in 0..num_calls {
            timestamps.push(unix());
            if num_calls > 10 {
                std::thread::sleep(std::time::Duration::from_nanos(100));
            }
        }

        // Verify all timestamps are in ascending order (or equal for very fast calls)
        for i in 1..timestamps.len() {
            assert!(
                timestamps[i] >= timestamps[i - 1],
                "Timestamp at index {i} ({:?}) should be >= previous timestamp ({:?})",
                timestamps[i],
                timestamps[i - 1]
            );
        }
    }

    // Test that the timestamp is reasonably current
    #[test]
    fn test_unix_timestamp_currency() {
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("getting current time before test must not fail");

        let result = unix();

        let after = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("getting current time after test must not fail");

        assert!(
            result >= before && result <= after,
            "Unix timestamp {result:?} should be between {before:?} and {after:?}"
        );
    }

    // Test nanosecond precision
    #[test]
    fn test_unix_nanosecond_precision() {
        let result = unix();

        // Should have subsecond precision (nanoseconds should not always be 0)
        // We'll check this by taking multiple samples
        let mut has_subsecond_precision = false;

        for _ in 0..10 {
            let sample = unix();
            if sample.subsec_nanos() != 0 {
                has_subsecond_precision = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_nanos(1));
        }

        // Note: This might occasionally fail on very slow systems, but should generally pass
        assert!(
            has_subsecond_precision || result.subsec_nanos() != 0,
            "Unix timestamp should have nanosecond precision"
        );
    }

    // Test performance
    #[rstest]
    #[case::performance_single(1)]
    #[case::performance_burst(100)]
    #[case::performance_intensive(1000)]
    fn test_unix_performance(#[case] iterations: usize) {
        let start = std::time::Instant::now();

        for _ in 0..iterations {
            let _ = unix();
        }

        let duration = start.elapsed();
        let max_duration = std::time::Duration::from_millis(if iterations == 1 {
            5
        } else if iterations <= 100 {
            50
        } else {
            200
        });

        assert!(
            duration < max_duration,
            "Function took too long: {duration:?} for {iterations} iterations"
        );
    }

    // Test concurrent safety
    #[test]
    fn test_unix_concurrent_safety() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for _ in 0..8 {
            let counter_clone = Arc::clone(&counter);
            let handle = std::thread::spawn(move || {
                for _ in 0..20 {
                    let _result = unix();
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("thread should not panic");
        }

        assert_eq!(
            counter.load(Ordering::SeqCst),
            160,
            "Not all concurrent calls completed successfully"
        );
    }

    // Test that function doesn't panic under normal conditions
    #[rstest]
    #[case::no_panic_repeated(50)]
    fn test_unix_no_panic(#[case] iterations: usize) {
        for i in 0..iterations {
            let result = std::panic::catch_unwind(unix);
            assert!(result.is_ok(), "Function panicked on iteration {}", i + 1);
        }
    }

    // Test Duration properties
    #[test]
    fn test_unix_duration_properties() {
        let result = unix();

        // Should be positive
        assert!(result.as_secs() > 0, "Unix timestamp should be positive");

        // Should have reasonable bounds for seconds component
        assert!(
            result.as_secs() < u64::MAX / 2,
            "Unix timestamp seconds should be reasonable"
        );

        // Nanoseconds should be valid
        assert!(
            result.subsec_nanos() < 1_000_000_000,
            "Nanoseconds component should be < 1 billion"
        );
    }
}
