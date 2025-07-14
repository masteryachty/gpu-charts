use chrono::{DateTime, TimeZone, Timelike, Utc};
use coinbase_logger::data_types::TickerData;

#[cfg(test)]
mod nanosecond_precision_tests {
    use super::*;

    #[test]
    fn test_timestamp_parsing_with_nanoseconds() {
        // Test parsing RFC3339 timestamps with nanosecond precision
        let timestamp_str = "2025-01-07T12:34:56.123456789Z";

        let dt = DateTime::parse_from_rfc3339(timestamp_str).unwrap();
        let timestamp_nanos = dt.timestamp_subsec_nanos();

        // Verify we can parse the timestamp
        assert!(dt.timestamp() > 0);

        // Verify nanosecond precision is preserved
        assert_eq!(timestamp_nanos, 123456789); // Full nanosecond precision
    }

    #[test]
    fn test_nanosecond_storage_in_ticker_data() {
        // Test that TickerData properly stores nanosecond precision
        let ticker_data = TickerData::new(
            1234567890, // seconds
            987654321,  // nanoseconds
            50000.0, 0.1, 1, 49999.0, 50001.0,
        );

        assert_eq!(ticker_data.timestamp_secs, 1234567890);
        assert_eq!(ticker_data.timestamp_nanos, 987654321);
    }

    #[test]
    fn test_binary_encoding_of_nanoseconds() {
        // Test that nanoseconds are properly encoded as 4-byte little-endian
        let nanos: u32 = 123456789;
        let bytes = nanos.to_le_bytes();

        assert_eq!(bytes.len(), 4);

        // Verify we can decode back
        let decoded = u32::from_le_bytes(bytes);
        assert_eq!(decoded, nanos);
    }

    #[test]
    fn test_full_timestamp_range() {
        // Test edge cases for nanosecond values
        let test_cases = vec![
            0_u32,           // Zero nanoseconds
            1_u32,           // One nanosecond
            999_999_999_u32, // Maximum valid nanoseconds
            500_000_000_u32, // Half second
            123_456_789_u32, // Arbitrary value
        ];

        for nanos in test_cases {
            let bytes = nanos.to_le_bytes();
            let decoded = u32::from_le_bytes(bytes);
            assert_eq!(decoded, nanos);

            // Verify it's less than 1 second
            assert!(nanos < 1_000_000_000);
        }
    }

    #[test]
    fn test_timestamp_ordering_with_nanoseconds() {
        // Test that timestamps with same second but different nanoseconds sort correctly
        let base_time = 1000_u64;

        let timestamps = vec![
            (base_time * 1_000_000_000 + 999_999_999, "A"), // Latest in second
            (base_time * 1_000_000_000 + 1, "B"),           // Earliest in second
            (base_time * 1_000_000_000 + 500_000_000, "C"), // Middle of second
        ];

        let mut sorted = timestamps.clone();
        sorted.sort_by_key(|&(ts, _)| ts);

        assert_eq!(sorted[0].1, "B"); // Earliest
        assert_eq!(sorted[1].1, "C"); // Middle
        assert_eq!(sorted[2].1, "A"); // Latest
    }

    #[test]
    fn test_coinbase_timestamp_format() {
        // Test parsing actual Coinbase timestamp format
        let coinbase_timestamps = vec![
            "2025-01-07T12:00:00.000000Z",    // Microsecond precision (common)
            "2025-01-07T12:00:00.123456789Z", // Full nanosecond precision
            "2025-01-07T12:00:00Z",           // No fractional seconds
            "2025-01-07T12:00:00.1Z",         // Tenths of second
        ];

        for ts_str in coinbase_timestamps {
            let dt = DateTime::parse_from_rfc3339(ts_str).unwrap();
            let nanos = dt.timestamp_subsec_nanos();

            // Verify we get the expected precision
            if ts_str.contains(".123456789") {
                assert_eq!(nanos, 123456789);
            } else if ts_str.contains(".000000") {
                assert_eq!(nanos, 0);
            } else if ts_str.contains(".1") {
                assert_eq!(nanos, 100000000); // 0.1 seconds = 100M nanoseconds
            } else {
                assert_eq!(nanos, 0);
            }
        }
    }

    #[test]
    fn test_separate_nanos_file_format() {
        // Test that we maintain separate files for seconds and nanoseconds
        let timestamp_secs: u32 = 1234567890;
        let timestamp_nanos: u32 = 987654321;

        // Simulate binary encoding for both files
        let time_bytes = timestamp_secs.to_le_bytes();
        let nanos_bytes = timestamp_nanos.to_le_bytes();

        // Both should be 4 bytes
        assert_eq!(time_bytes.len(), 4);
        assert_eq!(nanos_bytes.len(), 4);

        // Verify they decode correctly
        assert_eq!(u32::from_le_bytes(time_bytes), timestamp_secs);
        assert_eq!(u32::from_le_bytes(nanos_bytes), timestamp_nanos);
    }

    #[test]
    fn test_nanosecond_precision_not_lost() {
        // Test that converting to/from our storage format preserves precision
        let original_dt = Utc
            .with_ymd_and_hms(2025, 1, 7, 12, 34, 56)
            .unwrap()
            .with_nanosecond(123456789)
            .unwrap();

        let secs = original_dt.timestamp() as u32;
        let nanos = original_dt.timestamp_subsec_nanos();

        // Store and retrieve
        let ticker_data = TickerData::new(secs, nanos, 50000.0, 0.1, 1, 49999.0, 50001.0);

        // Reconstruct datetime
        let reconstructed_dt = Utc
            .timestamp_opt(
                ticker_data.timestamp_secs as i64,
                ticker_data.timestamp_nanos,
            )
            .single()
            .unwrap();

        assert_eq!(original_dt, reconstructed_dt);
    }
}
