use std::fs;
use std::io::Write;
use tempfile::TempDir;
use ultra_low_latency_server_chunked_parallel::{
    find_end_index, find_start_index, get_record_size, load_mmap, parse_query_params,
};

#[tokio::test]
async fn test_parse_query_params_valid() {
    let query = "symbol=BTC-USD&type=MD&start=1234567890&end=1234567900&columns=time,best_bid";
    let result = parse_query_params(Some(query));

    assert!(result.is_ok());
    let params = result.unwrap();
    assert_eq!(params.symbol, "BTC-USD");
    assert_eq!(params.type_, "MD");
    assert_eq!(params.start, 1234567890);
    assert_eq!(params.end, 1234567900);
    assert_eq!(params.columns, vec!["time", "best_bid"]);
}

#[tokio::test]
async fn test_parse_query_params_missing_symbol() {
    let query = "type=MD&start=1234567890&end=1234567900&columns=time,best_bid";
    let result = parse_query_params(Some(query));

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing symbol"));
}

#[tokio::test]
async fn test_parse_query_params_invalid_start() {
    let query = "symbol=BTC-USD&type=MD&start=invalid&end=1234567900&columns=time,best_bid";
    let result = parse_query_params(Some(query));

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid start"));
}

#[tokio::test]
async fn test_get_record_size_known_columns() {
    assert_eq!(get_record_size("time"), Some(4));
    assert_eq!(get_record_size("price"), Some(4));
    assert_eq!(get_record_size("best_bid"), Some(4));
    assert_eq!(get_record_size("best_ask"), Some(4));
    assert_eq!(get_record_size("volume"), Some(4));
    assert_eq!(get_record_size("side"), Some(4));
}

#[tokio::test]
async fn test_get_record_size_unknown_column() {
    assert_eq!(get_record_size("unknown_column"), Some(4)); // default
}

#[tokio::test]
async fn test_find_start_index_exact_match() {
    let time_data = vec![100u32, 200, 300, 400, 500];
    assert_eq!(find_start_index(&time_data, 300), 2);
}

#[tokio::test]
async fn test_find_start_index_between_elements() {
    let time_data = vec![100u32, 200, 300, 400, 500];
    assert_eq!(find_start_index(&time_data, 250), 2); // Should return index 2 (300)
}

#[tokio::test]
async fn test_find_start_index_before_first() {
    let time_data = vec![100u32, 200, 300, 400, 500];
    assert_eq!(find_start_index(&time_data, 50), 0);
}

#[tokio::test]
async fn test_find_start_index_after_last() {
    let time_data = vec![100u32, 200, 300, 400, 500];
    assert_eq!(find_start_index(&time_data, 600), 5);
}

#[tokio::test]
async fn test_find_end_index_exact_match() {
    let time_data = vec![100u32, 200, 300, 400, 500];
    assert_eq!(find_end_index(&time_data, 300), 2);
}

#[tokio::test]
async fn test_find_end_index_between_elements() {
    let time_data = vec![100u32, 200, 300, 400, 500];
    assert_eq!(find_end_index(&time_data, 250), 1); // Should return index 1 (200)
}

#[tokio::test]
async fn test_find_end_index_before_first() {
    let time_data = vec![100u32, 200, 300, 400, 500];
    assert_eq!(find_end_index(&time_data, 50), 0);
}

#[tokio::test]
async fn test_find_end_index_after_last() {
    let time_data = vec![100u32, 200, 300, 400, 500];
    assert_eq!(find_end_index(&time_data, 600), 4); // Should return index 4 (500)
}

#[tokio::test]
async fn test_load_mmap_success() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.bin");

    // Create test data: 5 u32 values
    let test_data = vec![1u32, 2, 3, 4, 5];
    let mut file = fs::File::create(&file_path).unwrap();
    let bytes: Vec<u8> = test_data
        .iter()
        .flat_map(|&x| x.to_le_bytes().to_vec())
        .collect();
    file.write_all(&bytes).unwrap();

    let result = load_mmap(file_path.to_str().unwrap()).await;
    assert!(result.is_ok());

    let mmap = result.unwrap();
    assert_eq!(mmap.len(), 20); // 5 * 4 bytes

    // Verify the data can be read back correctly
    let slice: &[u32] = unsafe { std::slice::from_raw_parts(mmap.as_ptr() as *const u32, 5) };
    assert_eq!(slice, &[1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_load_mmap_nonexistent_file() {
    let result = load_mmap("/nonexistent/path/file.bin").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Failed to open"));
}

#[tokio::test]
async fn test_load_mmap_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.bin");

    // Create empty file
    fs::File::create(&file_path).unwrap();

    let result = load_mmap(file_path.to_str().unwrap()).await;
    assert!(result.is_ok());

    let mmap = result.unwrap();
    assert_eq!(mmap.len(), 0);
}

#[test]
fn test_query_params_multiple_columns() {
    let query = "symbol=ETH-USD&type=MD&start=1000&end=2000&columns=time,best_bid,best_ask,volume";
    let result = parse_query_params(Some(query));

    assert!(result.is_ok());
    let params = result.unwrap();
    assert_eq!(params.columns.len(), 4);
    assert_eq!(
        params.columns,
        vec!["time", "best_bid", "best_ask", "volume"]
    );
}

#[test]
fn test_edge_case_single_element_array() {
    let time_data = vec![100u32];

    assert_eq!(find_start_index(&time_data, 100), 0);
    assert_eq!(find_start_index(&time_data, 50), 0);
    assert_eq!(find_start_index(&time_data, 150), 1);

    assert_eq!(find_end_index(&time_data, 100), 0);
    assert_eq!(find_end_index(&time_data, 50), 0);
    assert_eq!(find_end_index(&time_data, 150), 0);
}
