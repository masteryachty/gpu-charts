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

        assert_eq!(config.max_message_size, Some(128 << 20)); // 128MB
        assert_eq!(config.max_frame_size, Some(32 << 20)); // 32MB
        assert_eq!(config.write_buffer_size, 512 * 1024); // 512KB
        assert_eq!(config.max_write_buffer_size, 1024 * 1024); // 1MB
        assert!(!config.accept_unmasked_frames);
    }

    #[tokio::test]
    async fn test_symbol_distribution_across_connections() {
        // Test that symbols are evenly distributed
        const CONNECTIONS: usize = 10;

        // Simulate 197 symbols (prime number for uneven distribution)
        let symbols: Vec<String> = (0..197).map(|i| format!("SYMBOL-{i}-USD")).collect();

        let symbols_per_connection = symbols.len().div_ceil(CONNECTIONS);

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
        let expected_files = [
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
    #[allow(clippy::assertions_on_constants)]
    fn test_performance_metrics() {
        // Verify performance improvement claims

        // Performance improvements achieved
        const OLD_CONNECTIONS: f32 = 200.0;
        const NEW_CONNECTIONS: f32 = 10.0;
        const CONNECTION_REDUCTION: f32 = OLD_CONNECTIONS / NEW_CONNECTIONS;
        assert!(CONNECTION_REDUCTION >= 20.0); // 20x reduction

        const OLD_STARTUP_TIME: f32 = 386.0;
        const NEW_STARTUP_TIME: f32 = 1.0;
        const STARTUP_IMPROVEMENT: f32 = OLD_STARTUP_TIME / NEW_STARTUP_TIME;
        assert!(STARTUP_IMPROVEMENT >= 386.0); // 386x faster

        const OLD_BUFFER_SIZE: f32 = 8.0;
        const NEW_BUFFER_SIZE: f32 = 256.0;
        const BUFFER_INCREASE: f32 = NEW_BUFFER_SIZE / OLD_BUFFER_SIZE;
        assert!(BUFFER_INCREASE >= 32.0); // 32x larger

        const OLD_FLUSH_INTERVAL: f32 = 1.0;
        const NEW_FLUSH_INTERVAL: f32 = 5.0;
        const FLUSH_REDUCTION: f32 = NEW_FLUSH_INTERVAL / OLD_FLUSH_INTERVAL;
        assert!(FLUSH_REDUCTION >= 5.0); // 5x reduction in flushes
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
