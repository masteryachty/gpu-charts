use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::convert::Infallible;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

use hyper::{body::Body, header, Response, StatusCode};

#[derive(Debug, Serialize, Deserialize)]
struct SymbolInfo {
    symbol: String,
    last_update: u64,         // Unix timestamp
    last_update_date: String, // Human-readable date
}

/// Convert Unix timestamp to readable date string
fn timestamp_to_readable(timestamp: u64) -> String {
    if timestamp == 0 {
        return "Never".to_string();
    }

    match UNIX_EPOCH.checked_add(std::time::Duration::from_secs(timestamp)) {
        Some(time) => {
            let datetime: DateTime<Utc> = time.into();
            datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
        }
        None => "Invalid timestamp".to_string(),
    }
}

/// Get the most recent modification time of any file in the given directory
async fn get_latest_modification_time(
    base_path: &str,
    exchange: &str,
    symbol: &str,
) -> Option<u64> {
    let symbol_path = format!("{base_path}/{exchange}/{symbol}");

    // Look for any subdirectories (like MD, TRADES, etc.)
    match fs::read_dir(&symbol_path).await {
        Ok(mut type_dirs) => {
            let mut latest_time = 0u64;

            while let Some(type_entry) = type_dirs.next_entry().await.ok().flatten() {
                if let Ok(metadata) = type_entry.metadata().await {
                    if metadata.is_dir() {
                        let type_path = type_entry.path();

                        // Look for .bin files in this directory
                        if let Ok(mut bin_files) = fs::read_dir(&type_path).await {
                            while let Some(bin_entry) = bin_files.next_entry().await.ok().flatten()
                            {
                                if let Some(file_name) = bin_entry.file_name().to_str() {
                                    if file_name.ends_with(".bin") {
                                        if let Ok(file_metadata) = bin_entry.metadata().await {
                                            if let Ok(modified) = file_metadata.modified() {
                                                if let Ok(duration) =
                                                    modified.duration_since(SystemTime::UNIX_EPOCH)
                                                {
                                                    latest_time =
                                                        latest_time.max(duration.as_secs());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if latest_time > 0 {
                Some(latest_time)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

/// Handler for the /api/symbols endpoint.
pub async fn handle_symbols_request(
    req: hyper::Request<Body>,
) -> Result<Response<Body>, Infallible> {
    // Parse query parameters
    let query = req.uri().query().unwrap_or("");
    let params: HashMap<String, String> = url::form_urlencoded::parse(query.as_bytes())
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let exchange_filter = params.get("exchange").cloned();
    // Always use /mnt/md/data as the base path
    let base_path = "/mnt/md/data".to_string();
    let mut all_symbols = Vec::new();
    let mut exchanges_with_info: HashMap<String, Vec<SymbolInfo>> = HashMap::new();

    match fs::read_dir(&base_path).await {
        Ok(read_dir) => {
            let mut stream = ReadDirStream::new(read_dir);

            // First level: exchanges
            while let Some(entry_result) = stream.next().await {
                if let Ok(entry) = entry_result {
                    if let Ok(metadata) = entry.metadata().await {
                        if metadata.is_dir() {
                            if let Some(exchange_name) = entry.file_name().to_str() {
                                // Skip if we have a filter and this exchange doesn't match
                                if let Some(ref filter) = exchange_filter {
                                    if exchange_name != filter {
                                        continue;
                                    }
                                }
                                let mut exchange_symbols = Vec::new();
                                let exchange_path = format!("{base_path}/{exchange_name}");

                                // Second level: symbols
                                if let Ok(symbol_dir) = fs::read_dir(&exchange_path).await {
                                    let mut symbol_stream = ReadDirStream::new(symbol_dir);

                                    while let Some(symbol_entry_result) = symbol_stream.next().await
                                    {
                                        if let Ok(symbol_entry) = symbol_entry_result {
                                            if let Ok(symbol_metadata) =
                                                symbol_entry.metadata().await
                                            {
                                                if symbol_metadata.is_dir() {
                                                    if let Some(symbol_name) =
                                                        symbol_entry.file_name().to_str()
                                                    {
                                                        // Get the last update time for this symbol
                                                        let last_update =
                                                            get_latest_modification_time(
                                                                &base_path,
                                                                exchange_name,
                                                                symbol_name,
                                                            )
                                                            .await
                                                            .unwrap_or(0);

                                                        let symbol_info = SymbolInfo {
                                                            symbol: symbol_name.to_string(),
                                                            last_update,
                                                            last_update_date: timestamp_to_readable(
                                                                last_update,
                                                            ),
                                                        };

                                                        exchange_symbols.push(symbol_info);
                                                        all_symbols.push(symbol_name.to_string());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Sort symbols by last_update (newest first)
                                exchange_symbols.sort_by(|a, b| b.last_update.cmp(&a.last_update));

                                exchanges_with_info
                                    .insert(exchange_name.to_string(), exchange_symbols);
                            }
                        }
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to read data directory: {err}");
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to read symbol directory"))
                .unwrap());
        }
    }

    // Remove duplicates from all_symbols
    all_symbols.sort();
    all_symbols.dedup();

    let json = json!({
        "symbols": all_symbols,
        "exchanges": exchanges_with_info
    });
    let body = Body::from(json.to_string());

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(body)
        .unwrap())
}
