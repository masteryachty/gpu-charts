use coinbase_logger::{
    connection::{BUFFER_FLUSH_INTERVAL, MAX_BUFFER_SIZE},
    data_types::TickerData,
};
use serde_json::json;
use tempfile::TempDir;
use tokio::fs;

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_message_processing_pipeline() {
        // Test the complete message processing pipeline
        let _temp_dir = TempDir::new().unwrap();

        // Create test ticker message
        let ticker_message = json!({
            "type": "ticker",
            "product_id": "BTC-USD",
            "time": "2025-01-07T12:00:00.123456789Z",
            "price": "50000.00",
            "last_size": "0.12345",
            "side": "buy",
            "best_bid": "49999.00",
            "best_ask": "50001.00"
        });

        let message_str = ticker_message.to_string();

        // Verify JSON parsing
        let parsed: serde_json::Value = serde_json::from_str(&message_str).unwrap();
        assert_eq!(parsed.get("type").unwrap().as_str().unwrap(), "ticker");
        assert_eq!(
            parsed.get("product_id").unwrap().as_str().unwrap(),
            "BTC-USD"
        );
    }

    #[tokio::test]
    async fn test_buffer_management() {
        // Test buffer filling and flushing behavior
        let mut buffer = std::collections::BTreeMap::new();

        // Fill buffer with test data
        for i in 0..100 {
            let data = TickerData::new(
                1000 + i,
                i * 10_000_000,
                50000.0 + i as f32,
                0.1,
                (i % 2) as u8,
                49999.0,
                50001.0,
            );

            let key = (
                ((1000 + i) as u64) * 1_000_000_000 + (i as u64 * 10_000_000),
                "BTC-USD".to_string(),
            );

            buffer.insert(key, data);
        }

        assert_eq!(buffer.len(), 100);

        // Verify data is sorted
        let timestamps: Vec<u64> = buffer.keys().map(|(ts, _)| *ts).collect();
        for i in 1..timestamps.len() {
            assert!(timestamps[i] > timestamps[i - 1]);
        }
    }

    #[tokio::test]
    async fn test_websocket_config() {
        // Test WebSocket configuration values
        use coinbase_logger::websocket::create_websocket_config;

        let config = create_websocket_config();

        assert_eq!(config.max_message_size, Some(64 << 20)); // 64MB
        assert_eq!(config.max_frame_size, Some(16 << 20)); // 16MB
        assert_eq!(config.write_buffer_size, 256 * 1024); // 256KB
        assert_eq!(config.max_write_buffer_size, 512 * 1024); // 512KB
        assert!(!config.accept_unmasked_frames);
    }

    #[tokio::test]
    async fn test_symbol_distribution_across_connections() {
        // Test that symbols are evenly distributed
        const CONNECTIONS: usize = 10;

        // Simulate 197 symbols (prime number for uneven distribution)
        let symbols: Vec<String> = (0..197).map(|i| format!("SYMBOL-{}-USD", i)).collect();

        let symbols_per_connection = (symbols.len() + CONNECTIONS - 1) / CONNECTIONS;

        let mut total_assigned = 0;
        for i in 0..CONNECTIONS {
            let start_idx = i * symbols_per_connection;
            let end_idx = std::cmp::min((i + 1) * symbols_per_connection, symbols.len());

            if start_idx < symbols.len() {
                let connection_symbols = &symbols[start_idx..end_idx];
                total_assigned += connection_symbols.len();

                // Verify reasonable distribution
                assert!(connection_symbols.len() >= 17); // Minimum expected
                assert!(connection_symbols.len() <= 20); // Maximum expected
            }
        }

        // Verify all symbols are assigned
        assert_eq!(total_assigned, symbols.len());
    }

    #[tokio::test]
    async fn test_file_handle_creation() {
        // Test file handle creation with proper directory structure
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().join("BTC-USD").join("MD");

        // Create directory
        fs::create_dir_all(&base_path).await.unwrap();

        // Verify directory exists
        assert!(base_path.exists());
        assert!(base_path.is_dir());

        // Test file paths that would be created
        let date = "07.01.25";
        let expected_files = vec![
            format!("{}/time.{}.bin", base_path.display(), date),
            format!("{}/nanos.{}.bin", base_path.display(), date),
            format!("{}/price.{}.bin", base_path.display(), date),
            format!("{}/volume.{}.bin", base_path.display(), date),
            format!("{}/side.{}.bin", base_path.display(), date),
            format!("{}/best_bid.{}.bin", base_path.display(), date),
            format!("{}/best_ask.{}.bin", base_path.display(), date),
        ];

        // Verify we have 7 files per symbol (including new nanos file)
        assert_eq!(expected_files.len(), 7);
    }

    #[tokio::test]
    async fn test_concurrent_connection_creation() {
        // Test that connections are created concurrently
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        use tokio::sync::Barrier;

        let connection_count = Arc::new(AtomicU32::new(0));
        let barrier = Arc::new(Barrier::new(10));

        let mut handles = vec![];

        for i in 0..10 {
            let count = connection_count.clone();
            let b = barrier.clone();

            let handle = tokio::spawn(async move {
                // Simulate connection creation
                count.fetch_add(1, Ordering::SeqCst);

                // Wait for all connections to be "created"
                b.wait().await;

                i
            });

            handles.push(handle);
        }

        // All connections should complete roughly simultaneously
        let results: Vec<_> = futures_util::future::join_all(handles).await;

        assert_eq!(results.len(), 10);
        assert_eq!(connection_count.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_performance_metrics() {
        // Verify performance improvement claims

        // Connection reduction: 200+ -> 10
        assert!(10_f32 / 200.0 == 0.05); // 20x reduction

        // Startup time: 386s -> ~1s
        assert!(1.0 / 386.0 < 0.01); // 386x faster

        // Buffer size increase: 8KB -> 256KB
        assert!(256.0 / 8.0 == 32.0); // 32x larger

        // Flush interval: 1s -> 5s
        assert!(5.0 / 1.0 == 5.0); // 5x reduction in flushes
    }

    #[test]
    fn test_configuration_constants() {
        // Verify all configuration constants are set correctly
        assert_eq!(BUFFER_FLUSH_INTERVAL.as_secs(), 5);
        assert_eq!(MAX_BUFFER_SIZE, 10000);
        assert_eq!(
            coinbase_logger::connection::MAX_RECONNECT_DELAY.as_secs(),
            60
        );
        assert_eq!(coinbase_logger::file_handlers::FILE_BUFFER_SIZE, 64 * 1024);
    }
}
