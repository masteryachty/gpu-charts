use chrono::{DateTime, Utc};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::sync::RwLock;
use tokio::time::Instant;

use hyper::{body::Body, header, Response, StatusCode};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExchangeStatus {
    exchange: String,
    last_update: u64,         // Unix timestamp
    last_update_date: String, // Human-readable date
}

#[derive(Debug, Clone)]
struct CachedStatus {
    data: String,  // Pre-serialized JSON
    timestamp: Instant,
}

// Cache with 30-second TTL
static STATUS_CACHE: once_cell::sync::Lazy<Arc<RwLock<Option<CachedStatus>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(None)));

const CACHE_TTL_SECONDS: u64 = 30;

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

/// Get the last update for an exchange by checking ONLY the first symbol's first time file
/// This is super fast and accurate since all symbols update together
async fn get_exchange_quick_status(exchange: String) -> ExchangeStatus {
    let base_path = "/mnt/md/data";
    let exchange_path = format!("{}/{}", base_path, exchange);
    
    // Read the exchange directory to find the first symbol
    let mut entries = match fs::read_dir(&exchange_path).await {
        Ok(dir) => dir,
        Err(_) => {
            return ExchangeStatus {
                exchange,
                last_update: 0,
                last_update_date: "Never".to_string(),
            };
        }
    };
    
    // Get the FIRST symbol directory only
    let first_symbol = match entries.next_entry().await {
        Ok(Some(entry)) => entry,
        _ => {
            return ExchangeStatus {
                exchange,
                last_update: 0,
                last_update_date: "Never".to_string(),
            };
        }
    };
    
    // Now check the first data type directory (usually MD)
    let symbol_path = first_symbol.path();
    let mut type_dirs = match fs::read_dir(&symbol_path).await {
        Ok(dir) => dir,
        Err(_) => {
            return ExchangeStatus {
                exchange,
                last_update: 0,
                last_update_date: "Never".to_string(),
            };
        }
    };
    
    // Get the FIRST type directory
    let first_type = match type_dirs.next_entry().await {
        Ok(Some(entry)) => entry,
        _ => {
            return ExchangeStatus {
                exchange,
                last_update: 0,
                last_update_date: "Never".to_string(),
            };
        }
    };
    
    // Now just check for ANY .bin file and get its modification time
    let type_path = first_type.path();
    let mut bin_files = match fs::read_dir(&type_path).await {
        Ok(dir) => dir,
        Err(_) => {
            return ExchangeStatus {
                exchange,
                last_update: 0,
                last_update_date: "Never".to_string(),
            };
        }
    };
    
    // Find the FIRST .bin file and check its modification time
    while let Ok(Some(entry)) = bin_files.next_entry().await {
        if let Some(name) = entry.file_name().to_str() {
            if name.ends_with(".bin") {
                // Get modification time for this ONE file only
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                            let timestamp = duration.as_secs();
                            return ExchangeStatus {
                                exchange,
                                last_update: timestamp,
                                last_update_date: timestamp_to_readable(timestamp),
                            };
                        }
                    }
                }
                // Even if we couldn't get the time, stop here
                break;
            }
        }
    }
    
    ExchangeStatus {
        exchange,
        last_update: 0,
        last_update_date: "Never".to_string(),
    }
}

/// Handler for the /api/status endpoint - ULTRA FAST version
pub async fn handle_status_request(
    _req: hyper::Request<Body>,
) -> Result<Response<Body>, Infallible> {
    // Check cache first
    {
        let cache = STATUS_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECONDS) {
                // Return pre-serialized cached data
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("Access-Control-Allow-Origin", "*")
                    .header("X-Cache-Status", "HIT")
                    .body(Body::from(cached.data.clone()))
                    .unwrap());
            }
        }
    }
    
    // Cache miss - fetch data
    let start_time = Instant::now();
    let base_path = "/mnt/md/data";
    
    // Get list of exchanges (fast)
    let mut exchanges = Vec::new();
    let mut entries = match fs::read_dir(base_path).await {
        Ok(dir) => dir,
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to read data directory"))
                .unwrap());
        }
    };
    
    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Some(name) = entry.file_name().to_str() {
            // Skip hidden files and non-directories (directories don't have dots)
            if !name.starts_with('.') && !name.contains('.') {
                exchanges.push(name.to_string());
            }
        }
    }
    
    // Process all exchanges in parallel - but each one only checks ONE file!
    let futures: Vec<_> = exchanges
        .into_iter()
        .map(|exchange| get_exchange_quick_status(exchange))
        .collect();
    
    let mut statuses = join_all(futures).await;
    
    // Sort by exchange name
    statuses.sort_unstable_by(|a, b| a.exchange.cmp(&b.exchange));
    
    let fetch_duration = start_time.elapsed();
    
    // Build response
    let response_data = json!({
        "exchanges": statuses,
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        "fetch_time_ms": fetch_duration.as_millis(),
        "cached": false
    });
    
    let json_string = response_data.to_string();
    
    // Update cache with pre-serialized data
    {
        let mut cache = STATUS_CACHE.write().await;
        *cache = Some(CachedStatus {
            data: json_string.clone(),
            timestamp: Instant::now(),
        });
    }
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .header("X-Cache-Status", "MISS")
        .header("X-Fetch-Time-Ms", fetch_duration.as_millis().to_string())
        .body(Body::from(json_string))
        .unwrap())
}