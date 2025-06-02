use time::{OffsetDateTime, format_description::well_known::Iso8601};

/// # Panics
/// This function panics when writing ISO 8601 datetime to string fails
#[must_use]
pub fn iso8601() -> String {
    OffsetDateTime::now_utc()
        .format(&Iso8601::DEFAULT)
        .expect("writing iso8601 datetime to string should never fail")
}

#[cfg(test)]
mod tests {
    use super::iso8601;
    use regex::Regex;
    use rstest::rstest;
    use std::thread;
    use time::OffsetDateTime;

    // Test that the function returns a valid ISO 8601 string format
    #[test]
    fn test_iso8601_format_validity() {
        let result = iso8601();

        // ISO 8601 regex pattern: YYYY-MM-DDTHH:MM:SS.sssZ or YYYY-MM-DDTHH:MM:SSZ
        let iso8601_regex = Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$")
            .expect("regex must be valid");

        assert!(
            iso8601_regex.is_match(&result),
            "Result '{result}' does not match ISO 8601 format"
        );
    }

    // Test that consecutive calls return timestamps in chronological order
    #[test]
    fn test_iso8601_chronological_order() {
        let first = iso8601();
        thread::sleep(std::time::Duration::from_millis(1));
        let second = iso8601();

        // Parse both timestamps to compare
        let first_dt = OffsetDateTime::parse(
            &first,
            &time::format_description::well_known::Iso8601::DEFAULT,
        )
        .expect("parsing datetime must not fail");
        let second_dt = OffsetDateTime::parse(
            &second,
            &time::format_description::well_known::Iso8601::DEFAULT,
        )
        .expect("parsing datetime must not fail");

        assert!(
            second_dt >= first_dt,
            "Second timestamp '{second}' should be >= first timestamp '{first}'"
        );
    }

    // Test function returns consistent format across multiple calls
    #[rstest]
    #[case::format_consistency(5)]
    #[case::format_consistency(10)]
    #[case::format_consistency(20)]
    fn test_iso8601_format_consistency(#[case] num_calls: usize) {
        let iso8601_regex = Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$")
            .expect("regex must be valid");

        for i in 0..num_calls {
            let result = iso8601();
            assert!(
                iso8601_regex.is_match(&result),
                "Call #{}: Result '{}' does not match ISO 8601 format",
                i + 1,
                result
            );
        }
    }

    // Test that returned string can be parsed back to OffsetDateTime
    #[test]
    fn test_iso8601_round_trip_parsing() {
        let iso_string = iso8601();

        let parsed_result = OffsetDateTime::parse(
            &iso_string,
            &time::format_description::well_known::Iso8601::DEFAULT,
        );

        assert!(
            parsed_result.is_ok(),
            "Failed to parse generated ISO 8601 string '{}': {:?}",
            iso_string,
            parsed_result.err()
        );
    }

    // Test that the timestamp is reasonably current (within last few seconds)
    #[test]
    fn test_iso8601_timestamp_currency() {
        let before = OffsetDateTime::now_utc();
        let iso_string = iso8601();
        let after = OffsetDateTime::now_utc();

        let parsed_timestamp = OffsetDateTime::parse(
            &iso_string,
            &time::format_description::well_known::Iso8601::DEFAULT,
        )
        .expect("parsing datetime must not fail");

        assert!(
            parsed_timestamp >= before && parsed_timestamp <= after,
            "Timestamp '{iso_string}' should be between '{before}' and '{after}'"
        );
    }

    // Test string length is within expected bounds
    #[test]
    fn test_iso8601_string_length() {
        let result = iso8601();

        // ISO 8601 with nanoseconds: 2024-01-01T12:00:00.123456789Z (30 chars)
        // ISO 8601 without subseconds: 2024-01-01T12:00:00Z (20 chars)
        assert!(
            result.len() >= 20 && result.len() <= 35,
            "ISO 8601 string length {} is outside expected bounds [20, 35]: '{}'",
            result.len(),
            result
        );
    }

    // Test UTC timezone indicator
    #[test]
    fn test_iso8601_utc_timezone() {
        let result = iso8601();

        assert!(
            result.ends_with('Z'),
            "ISO 8601 string '{result}' should end with 'Z' to indicate UTC"
        );
    }

    // Test specific components of the ISO 8601 format
    #[rstest]
    #[case::year_component(0)]
    #[case::month_component(1)]
    #[case::day_component(2)]
    #[case::hour_component(3)]
    #[case::minute_component(4)]
    #[case::second_component(5)]
    fn test_iso8601_component_validation(#[case] component_index: usize) {
        let result = iso8601();

        // Split by common delimiters to get components
        let replace = result.replace(['T', ':'], "-").replace('Z', "");
        let parts: Vec<&str> = replace
            .split('.')
            .next()
            .expect("parts must exist")
            .split('-')
            .collect();

        match component_index {
            0 => {
                // Year
                let year: i32 = parts[0].parse().expect("parsing must not fail");
                assert!(
                    (2000..=3000).contains(&year),
                    "Year {year} is out of reasonable range"
                );
            }
            1 => {
                // Month
                let month: u8 = parts[1].parse().expect("parsing must not fail");
                assert!((1..=12).contains(&month), "Month {month} is invalid");
            }
            2 => {
                // Day
                let day: u8 = parts[2].parse().expect("parsing must not fail");
                assert!((1..=31).contains(&day), "Day {day} is invalid");
            }
            3 => {
                // Hour
                let hour: u8 = parts[3].parse().expect("parsing must not fail");
                assert!(hour <= 23, "Hour {hour} is invalid");
            }
            4 => {
                // Minute
                let minute: u8 = parts[4].parse().expect("parsing must not fail");
                assert!(minute <= 59, "Minute {minute} is invalid");
            }
            5 => {
                // Second
                let second: u8 = parts[5].parse().expect("parsing must not fail");
                assert!(second <= 59, "Second {second} is invalid");
            }
            _ => panic!("Invalid component index"),
        }
    }

    // Performance test - function should complete quickly
    #[rstest]
    #[case::performance_single(1)]
    #[case::performance_burst(100)]
    fn test_iso8601_performance(#[case] iterations: usize) {
        let start = std::time::Instant::now();

        for _ in 0..iterations {
            let _ = iso8601();
        }

        let duration = start.elapsed();
        let max_duration = std::time::Duration::from_millis(if iterations == 1 { 10 } else { 100 });

        assert!(
            duration < max_duration,
            "Function took too long: {duration:?} for {iterations} iterations"
        );
    }

    // Test that function doesn't panic under normal conditions
    #[rstest]
    #[case::no_panic_single(1)]
    #[case::no_panic_multiple(50)]
    fn test_iso8601_no_panic(#[case] iterations: usize) {
        for i in 0..iterations {
            let result = std::panic::catch_unwind(iso8601);
            assert!(result.is_ok(), "Function panicked on iteration {}", i + 1);
        }
    }

    // Test concurrent access safety
    #[test]
    fn test_iso8601_concurrent_safety() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for _ in 0..10 {
            let counter_clone = Arc::clone(&counter);
            let handle = thread::spawn(move || {
                for _ in 0..10 {
                    let _result = iso8601();
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("joining threads must not fail");
        }

        assert_eq!(
            counter.load(Ordering::SeqCst),
            100,
            "Not all concurrent calls completed successfully"
        );
    }

    // Test different execution contexts (with slight delays)
    #[rstest]
    #[case::immediate(0)]
    #[case::after_short_delay(1)]
    #[case::after_medium_delay(10)]
    #[case::after_longer_delay(100)]
    fn test_iso8601_execution_contexts(#[case] delay_ms: u64) {
        if delay_ms > 0 {
            thread::sleep(std::time::Duration::from_millis(delay_ms));
        }

        let result = iso8601();

        // Verify it's still a valid ISO 8601 format regardless of delay
        let iso8601_regex = Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$")
            .expect("regex must be valid");

        assert!(
            iso8601_regex.is_match(&result),
            "Result '{result}' after {delay_ms}ms delay does not match ISO 8601 format"
        );
    }
}
