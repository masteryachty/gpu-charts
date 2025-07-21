//! Integration tests for the data manager

use gpu_charts_data::{DataManager, buffer_pool::BufferPool, cache::DataCache};
use gpu_charts_shared::{DataRequest, TimeRange, AggregationConfig, AggregationType};
use std::sync::Arc;

#[tokio::test]
async fn test_data_manager_creation() {
    // Mock device and queue creation for testing
    // In real tests, we'd use a test harness that provides these
    
    let base_url = "http://localhost:8080".to_string();
    
    // Test cache creation
    let cache = DataCache::new(1024 * 1024 * 1024); // 1GB
    assert_eq!(cache.get_stats()["entries"], 0);
}

#[test]
fn test_buffer_pool() {
    let mut pool = BufferPool::new(512 * 1024 * 1024); // 512MB
    
    // Test buffer size categorization
    let stats = pool.get_stats();
    assert_eq!(stats["small_buffers"], 0);
    assert_eq!(stats["medium_buffers"], 0);
    assert_eq!(stats["large_buffers"], 0);
    assert_eq!(stats["huge_buffers"], 0);
}

#[test]
fn test_data_request_serialization() {
    let request = DataRequest {
        symbol: "BTC-USD".to_string(),
        time_range: TimeRange::new(1000, 2000),
        columns: vec!["time".to_string(), "price".to_string()],
        aggregation: Some(AggregationConfig {
            aggregation_type: AggregationType::Ohlc,
            timeframe: 60,
        }),
        max_points: Some(10000),
    };
    
    // Test serialization round-trip
    let json = serde_json::to_string(&request).unwrap();
    let deserialized: DataRequest = serde_json::from_str(&json).unwrap();
    
    assert_eq!(request.symbol, deserialized.symbol);
    assert_eq!(request.time_range, deserialized.time_range);
    assert_eq!(request.columns, deserialized.columns);
}

#[test]
fn test_cache_key_generation() {
    use gpu_charts_data::cache::CacheKey;
    
    let request1 = DataRequest {
        symbol: "BTC-USD".to_string(),
        time_range: TimeRange::new(1000, 2000),
        columns: vec!["time".to_string(), "price".to_string()],
        aggregation: None,
        max_points: None,
    };
    
    let request2 = DataRequest {
        symbol: "BTC-USD".to_string(),
        time_range: TimeRange::new(1000, 2000),
        columns: vec!["price".to_string(), "time".to_string()], // Different order
        aggregation: None,
        max_points: None,
    };
    
    let key1 = CacheKey::from_request(&request1);
    let key2 = CacheKey::from_request(&request2);
    
    // Keys should be equal despite column order difference
    assert_eq!(key1, key2);
}

#[test]
fn test_binary_header_parsing() {
    use gpu_charts_data::parser::{BinaryParser, ColumnType};
    
    // Create test header data
    let mut data = Vec::new();
    
    // Magic number "GPCH"
    data.extend_from_slice(&0x47504348u32.to_le_bytes());
    // Header size
    data.extend_from_slice(&64u32.to_le_bytes());
    // Row count
    data.extend_from_slice(&1000u32.to_le_bytes());
    // Column count
    data.extend_from_slice(&2u32.to_le_bytes());
    
    // Column 1: "time"
    data.push(4); // name length
    data.extend_from_slice(b"time");
    data.push(0); // U32 type
    
    // Column 2: "price"
    data.push(5); // name length
    data.extend_from_slice(b"price");
    data.push(1); // F32 type
    
    // Pad to header size
    while data.len() < 64 {
        data.push(0);
    }
    
    // Test parsing
    let (header, header_size) = BinaryParser::parse_header(&data).unwrap();
    
    assert_eq!(header_size, 64);
    assert_eq!(header.row_count, 1000);
    assert_eq!(header.columns.len(), 2);
    assert_eq!(header.columns[0], "time");
    assert_eq!(header.columns[1], "price");
    assert!(matches!(header.column_types[0], ColumnType::U32));
    assert!(matches!(header.column_types[1], ColumnType::F32));
}

#[cfg(target_arch = "wasm32")]
mod wasm_tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    wasm_bindgen_test_configure!(run_in_browser);
    
    #[wasm_bindgen_test]
    async fn test_wasm_data_manager() {
        // Test WASM-specific functionality
        // This would run in a browser environment
    }
}