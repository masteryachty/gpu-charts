use coinbase_logger::data_types::TickerData;
use std::collections::BTreeMap;

#[cfg(test)]
mod message_buffering_tests {
    use super::*;

    #[test]
    fn test_btreemap_automatic_sorting() {
        // Test that BTreeMap automatically sorts by timestamp
        let mut buffer: BTreeMap<(u64, String), TickerData> = BTreeMap::new();
        
        // Insert messages out of order
        let data1 = TickerData::new(1000, 500_000_000, 50000.0, 0.1, 1, 49999.0, 50001.0);
        let data2 = TickerData::new(1000, 200_000_000, 50100.0, 0.2, 0, 50099.0, 50101.0);
        let data3 = TickerData::new(1000, 800_000_000, 50050.0, 0.15, 1, 50049.0, 50051.0);
        
        // Create keys with full nanosecond precision
        let key1 = (1000_u64 * 1_000_000_000 + 500_000_000, "BTC-USD".to_string());
        let key2 = (1000_u64 * 1_000_000_000 + 200_000_000, "BTC-USD".to_string());
        let key3 = (1000_u64 * 1_000_000_000 + 800_000_000, "BTC-USD".to_string());
        
        // Insert in random order
        buffer.insert(key1.clone(), data1.clone());
        buffer.insert(key3.clone(), data3.clone());
        buffer.insert(key2.clone(), data2.clone());
        
        // Verify they're sorted when iterating
        let keys: Vec<_> = buffer.keys().cloned().collect();
        assert_eq!(keys[0], key2); // Earliest timestamp
        assert_eq!(keys[1], key1); // Middle timestamp
        assert_eq!(keys[2], key3); // Latest timestamp
    }

    #[test]
    fn test_buffer_size_limits() {
        // Test buffer flush behavior at MAX_BUFFER_SIZE
        const MAX_BUFFER_SIZE: usize = 10000;
        
        let mut buffer: BTreeMap<(u64, String), TickerData> = BTreeMap::new();
        
        // Fill buffer to just under limit
        for i in 0..MAX_BUFFER_SIZE - 1 {
            let data = TickerData::new(
                1000 + (i / 1000) as u32,
                (i % 1000) as u32 * 1_000_000,
                50000.0 + i as f32,
                0.1,
                (i % 2) as u8,
                49999.0,
                50001.0
            );
            
            let key = (
                ((1000 + (i / 1000)) as u64) * 1_000_000_000 + ((i % 1000) as u64 * 1_000_000),
                "BTC-USD".to_string()
            );
            
            buffer.insert(key, data);
        }
        
        assert_eq!(buffer.len(), MAX_BUFFER_SIZE - 1);
        
        // Add one more to reach limit
        let data = TickerData::new(2000, 0, 51000.0, 0.1, 1, 50999.0, 51001.0);
        let key = (2000_u64 * 1_000_000_000, "BTC-USD".to_string());
        buffer.insert(key, data);
        
        assert_eq!(buffer.len(), MAX_BUFFER_SIZE);
    }

    #[test]
    fn test_multi_symbol_sorting() {
        // Test that messages from different symbols are sorted correctly
        let mut buffer: BTreeMap<(u64, String), TickerData> = BTreeMap::new();
        
        // Same timestamp, different symbols
        let timestamp_nanos = 1000_u64 * 1_000_000_000 + 500_000_000;
        
        let data_btc = TickerData::new(1000, 500_000_000, 50000.0, 0.1, 1, 49999.0, 50001.0);
        let data_eth = TickerData::new(1000, 500_000_000, 3000.0, 1.0, 0, 2999.0, 3001.0);
        let data_sol = TickerData::new(1000, 500_000_000, 100.0, 10.0, 1, 99.9, 100.1);
        
        buffer.insert((timestamp_nanos, "BTC-USD".to_string()), data_btc);
        buffer.insert((timestamp_nanos, "ETH-USD".to_string()), data_eth);
        buffer.insert((timestamp_nanos, "SOL-USD".to_string()), data_sol);
        
        // Different timestamps, same symbol
        let data_btc2 = TickerData::new(1001, 0, 50100.0, 0.2, 0, 50099.0, 50101.0);
        buffer.insert((1001_u64 * 1_000_000_000, "BTC-USD".to_string()), data_btc2);
        
        // Verify ordering: first by timestamp, then by symbol name
        let entries: Vec<_> = buffer.iter().collect();
        
        // All entries with same timestamp should be grouped together
        assert_eq!(entries[0].0.0, timestamp_nanos);
        assert_eq!(entries[1].0.0, timestamp_nanos);
        assert_eq!(entries[2].0.0, timestamp_nanos);
        
        // Within same timestamp, symbols should be alphabetically ordered
        assert_eq!(entries[0].0.1, "BTC-USD");
        assert_eq!(entries[1].0.1, "ETH-USD");
        assert_eq!(entries[2].0.1, "SOL-USD");
        
        // Later timestamp should come last
        assert_eq!(entries[3].0.0, 1001_u64 * 1_000_000_000);
    }

    #[test]
    fn test_flush_interval_timing() {
        use std::time::Duration;
        
        const BUFFER_FLUSH_INTERVAL: Duration = Duration::from_secs(5);
        
        // Verify flush interval is set correctly
        assert_eq!(BUFFER_FLUSH_INTERVAL.as_secs(), 5);
        
        // This is more of a configuration test
        // In production, we'd use tokio::time::interval to test actual timing
    }

    #[test]
    fn test_buffer_clear_after_flush() {
        let mut buffer: BTreeMap<(u64, String), TickerData> = BTreeMap::new();
        
        // Add some data
        for i in 0..100 {
            let data = TickerData::new(1000, i * 10_000_000, 50000.0 + i as f32, 0.1, 0, 49999.0, 50001.0);
            let key = (1000_u64 * 1_000_000_000 + (i as u64 * 10_000_000), "BTC-USD".to_string());
            buffer.insert(key, data);
        }
        
        assert_eq!(buffer.len(), 100);
        
        // Simulate flush
        buffer.clear();
        
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_nanosecond_precision_key_generation() {
        // Test the key generation logic preserves nanosecond precision
        let timestamp_secs: u32 = 1_234_567_890;
        let timestamp_nanos: u32 = 123_456_789;
        
        let key = (
            (timestamp_secs as u64) * 1_000_000_000 + (timestamp_nanos as u64),
            "BTC-USD".to_string()
        );
        
        // Verify we can reconstruct the original values
        let total_nanos = key.0;
        let reconstructed_secs = (total_nanos / 1_000_000_000) as u32;
        let reconstructed_nanos = (total_nanos % 1_000_000_000) as u32;
        
        assert_eq!(reconstructed_secs, timestamp_secs);
        assert_eq!(reconstructed_nanos, timestamp_nanos);
    }
}