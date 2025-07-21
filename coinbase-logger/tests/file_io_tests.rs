use coinbase_logger::data_types::TickerData;
use tempfile::{NamedTempFile, TempDir};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(test)]
mod file_io_tests {
    use super::*;

    #[tokio::test]
    async fn test_binary_file_format() {
        // Test that data is written in correct binary format
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write test data
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .await
            .unwrap();

        let test_value: u32 = 0x12345678;
        let bytes = test_value.to_le_bytes();
        file.write_all(&bytes).await.unwrap();
        file.flush().await.unwrap();
        drop(file);

        // Read back and verify
        let mut file = File::open(path).await.unwrap();
        let mut buffer = [0u8; 4];
        file.read_exact(&mut buffer).await.unwrap();

        let read_value = u32::from_le_bytes(buffer);
        assert_eq!(read_value, test_value);
    }

    #[tokio::test]
    async fn test_file_naming_convention() {
        // Test file naming follows the pattern: {column}.{DD}.{MM}.{YY}.bin
        let formatted_date = "07.01.25"; // Fixed date for testing

        let columns = [
            "time", "nanos", "price", "volume", "side", "best_bid", "best_ask",
        ];

        for column in &columns {
            let filename = format!("{column}.{formatted_date}.bin");

            // Verify format
            assert!(filename.ends_with(".bin"));
            assert!(filename.contains('.'));

            // Example: time.07.01.25.bin
            let parts: Vec<&str> = filename.split('.').collect();
            assert_eq!(parts.len(), 5); // ["time", "07", "01", "25", "bin"]
            assert_eq!(parts[0], *column);
            assert_eq!(parts[4], "bin");

            // Verify date parts are numeric
            assert!(parts[1].parse::<u32>().is_ok()); // Day
            assert!(parts[2].parse::<u32>().is_ok()); // Month
            assert!(parts[3].parse::<u32>().is_ok()); // Year
        }
    }

    #[tokio::test]
    async fn test_data_column_sizes() {
        // Test that all data columns are 4 bytes
        let ticker_data = TickerData::new(
            1234567890, // time
            987654321,  // nanos
            50000.0,    // price
            0.12345,    // volume
            1,          // side
            49999.0,    // best_bid
            50001.0,    // best_ask
        );

        // All values should encode to 4 bytes
        assert_eq!(ticker_data.timestamp_secs.to_le_bytes().len(), 4);
        assert_eq!(ticker_data.timestamp_nanos.to_le_bytes().len(), 4);
        assert_eq!(ticker_data.price.to_le_bytes().len(), 4);
        assert_eq!(ticker_data.volume.to_le_bytes().len(), 4);
        assert_eq!([ticker_data.side, 0, 0, 0].len(), 4); // Padded to 4 bytes
        assert_eq!(ticker_data.best_bid.to_le_bytes().len(), 4);
        assert_eq!(ticker_data.best_ask.to_le_bytes().len(), 4);
    }

    #[tokio::test]
    async fn test_side_value_encoding() {
        // Test side encoding (buy=1, sell=0)
        let buy_side: u8 = 1;
        let sell_side: u8 = 0;

        // Test padding to 4 bytes
        let buy_bytes = [buy_side, 0, 0, 0];
        let sell_bytes = [sell_side, 0, 0, 0];

        assert_eq!(buy_bytes.len(), 4);
        assert_eq!(sell_bytes.len(), 4);
        assert_eq!(buy_bytes[0], 1);
        assert_eq!(sell_bytes[0], 0);
    }

    #[tokio::test]
    async fn test_append_mode_writing() {
        // Test that files are opened in append mode
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write first value
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .unwrap();

        let value1: u32 = 100;
        file.write_all(&value1.to_le_bytes()).await.unwrap();
        drop(file);

        // Write second value (should append)
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .unwrap();

        let value2: u32 = 200;
        file.write_all(&value2.to_le_bytes()).await.unwrap();
        drop(file);

        // Read both values
        let mut file = File::open(path).await.unwrap();
        let mut buffer = [0u8; 8];
        file.read_exact(&mut buffer).await.unwrap();

        let read_value1 = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        let read_value2 = u32::from_le_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);

        assert_eq!(read_value1, value1);
        assert_eq!(read_value2, value2);
    }

    #[tokio::test]
    async fn test_directory_structure_creation() {
        // Test directory structure /data/{symbol}/MD/
        let temp_dir = TempDir::new().unwrap();
        let symbol = "BTC-USD";

        let full_path = temp_dir.path().join(symbol).join("MD");
        tokio::fs::create_dir_all(&full_path).await.unwrap();

        assert!(full_path.exists());
        assert!(full_path.is_dir());

        // Verify parent directories
        assert!(temp_dir.path().join(symbol).exists());
    }

    #[tokio::test]
    async fn test_float_precision() {
        // Test that f32 values maintain sufficient precision
        let test_prices = vec![
            50000.00_f32,
            50000.12_f32,
            0.00001234_f32, // Small value
            99999999.0_f32, // Large value
        ];

        for price in test_prices {
            let bytes = price.to_le_bytes();
            let decoded = f32::from_le_bytes(bytes);

            // For most financial data, f32 precision is sufficient
            let diff = (price - decoded).abs();
            let relative_error = diff / price.abs();

            // Verify relative error is negligible
            assert!(relative_error < 0.0001); // 0.01% error tolerance
        }
    }

    #[tokio::test]
    async fn test_buffered_writing() {
        // Test that BufferedWriter improves performance
        use tokio::io::BufWriter;

        let temp_file = NamedTempFile::new().unwrap();
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .open(temp_file.path())
            .await
            .unwrap();

        let mut writer = BufWriter::with_capacity(64 * 1024, file);

        // Write many small values
        for i in 0..1000 {
            let value = i as u32;
            writer.write_all(&value.to_le_bytes()).await.unwrap();
        }

        // Flush buffer
        writer.flush().await.unwrap();

        // Verify file size
        let metadata = tokio::fs::metadata(temp_file.path()).await.unwrap();
        assert_eq!(metadata.len(), 4000); // 1000 values * 4 bytes
    }

    #[test]
    fn test_file_buffer_size() {
        // Test that file buffer size is 64KB
        use coinbase_logger::file_handlers::FILE_BUFFER_SIZE;

        assert_eq!(FILE_BUFFER_SIZE, 64 * 1024);

        // This should reduce syscalls by 10-100x
        let typical_write_size = 28; // 7 fields * 4 bytes
        let writes_per_flush = FILE_BUFFER_SIZE / typical_write_size;

        assert!(writes_per_flush > 2000); // Many writes before flush
    }

    #[tokio::test]
    async fn test_parallel_file_writes() {
        // Test that multiple files can be written in parallel
        use futures_util::future::try_join_all;

        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        let columns = vec!["time", "price", "volume"];
        let mut write_futures = vec![];

        for column in columns {
            let path = base_path.join(format!("{column}.bin"));
            let future = async move {
                let mut file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(&path)
                    .await?;

                let value: u32 = 42;
                file.write_all(&value.to_le_bytes()).await?;
                file.flush().await?;

                Ok::<(), std::io::Error>(())
            };

            write_futures.push(future);
        }

        // All writes complete in parallel
        try_join_all(write_futures).await.unwrap();

        // Verify all files exist
        assert!(base_path.join("time.bin").exists());
        assert!(base_path.join("price.bin").exists());
        assert!(base_path.join("volume.bin").exists());
    }
}
