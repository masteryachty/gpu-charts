use std::fs;
use std::io::Write;
use tempfile::TempDir;
use ultra_low_latency_server_chunked_parallel::data::*;

#[tokio::test]
async fn test_parse_query_params() {
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
async fn test_parse_query_params_missing_fields() {
    let query = "symbol=BTC-USD&type=MD";
    let result = parse_query_params(Some(query));

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing start"));
}

#[tokio::test]
async fn test_get_record_size() {
    assert_eq!(get_record_size("time"), Some(4));
    assert_eq!(get_record_size("price"), Some(4));
    assert_eq!(get_record_size("best_bid"), Some(4));
    assert_eq!(get_record_size("best_ask"), Some(4));
    assert_eq!(get_record_size("volume"), Some(4));
    assert_eq!(get_record_size("side"), Some(4));
    assert_eq!(get_record_size("unknown"), Some(4)); // default
}

#[tokio::test]
async fn test_find_start_index() {
    let time_data = vec![100u32, 200, 300, 400, 500];

    // Exact match
    assert_eq!(find_start_index(&time_data, 300), 2);

    // Value between elements
    assert_eq!(find_start_index(&time_data, 250), 2);

    // Before first element
    assert_eq!(find_start_index(&time_data, 50), 0);

    // After last element
    assert_eq!(find_start_index(&time_data, 600), 5);
}

#[tokio::test]
async fn test_find_end_index() {
    let time_data = vec![100u32, 200, 300, 400, 500];

    // Exact match
    assert_eq!(find_end_index(&time_data, 300), 2);

    // Value between elements
    assert_eq!(find_end_index(&time_data, 250), 1);

    // Before first element
    assert_eq!(find_end_index(&time_data, 50), 0);

    // After last element
    assert_eq!(find_end_index(&time_data, 600), 4);
}

#[tokio::test]
async fn test_load_mmap_success() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.bin");

    // Create test data
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
}

#[tokio::test]
async fn test_load_mmap_nonexistent_file() {
    let result = load_mmap("/nonexistent/path/file.bin").await;
    assert!(result.is_err());
}

#[cfg(test)]
mod integration_tests {
    use hyper::{Body, Method, Request};
    use std::fs;
    use tempfile::TempDir;

    async fn create_test_data_structure() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().join("BTC-USD").join("MD");
        fs::create_dir_all(&base_path).unwrap();

        // Create time data for one day (01.01.25)
        let time_data = vec![1000u32, 2000, 3000, 4000, 5000];
        let time_bytes: Vec<u8> = time_data
            .iter()
            .flat_map(|&x| x.to_le_bytes().to_vec())
            .collect();
        fs::write(base_path.join("time.01.01.25.bin"), &time_bytes).unwrap();

        // Create best_bid data
        let bid_data = vec![100u32, 101, 102, 103, 104];
        let bid_bytes: Vec<u8> = bid_data
            .iter()
            .flat_map(|&x| x.to_le_bytes().to_vec())
            .collect();
        fs::write(base_path.join("best_bid.01.01.25.bin"), &bid_bytes).unwrap();

        temp_dir
    }

    #[tokio::test]
    async fn test_mock_data_request() {
        let _temp_dir = create_test_data_structure().await;

        // Note: This would require modifying the data.rs to accept a custom base path
        // for testing, or setting up environment variables

        // For now, just test that the function signature works
        let _request = Request::builder()
            .method(Method::GET)
            .uri("/api/data?symbol=BTC-USD&type=MD&start=1000&end=5000&columns=time,best_bid")
            .body(Body::empty())
            .unwrap();

        // This would require actual implementation to pass custom data path
        // let response = handle_data_request(request).await;
        // assert!(response.is_ok());
    }
}
