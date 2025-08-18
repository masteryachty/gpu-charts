use chrono::{DateTime, Utc, Datelike};
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
struct DataTypeStatus {
    last_update: u64,
    last_update_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExchangeStatus {
    exchange: String,
    last_update: u64,  // Most recent update across all data types
    last_update_date: String,  // Human-readable version of last_update
    md: DataTypeStatus,
    trades: DataTypeStatus,
    symbol_checked: String,  // Show which symbol was used for the check
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

/// Get the hardcoded BTC/USD symbol for each exchange
/// These are the most liquid pairs that should always be updating
fn get_btc_symbol_for_exchange(exchange: &str) -> &'static str {
    match exchange {
        "binance" => "BTCUSDT",    // Binance uses USDT as primary
        "coinbase" => "BTC-USD",   // Coinbase uses USD
        "kraken" => "XBT_USD",     // Kraken uses XBT for Bitcoin
        "bitfinex" => "tBTCUSD",   // Bitfinex uses tBTCUSD
        "okx" => "BTC-USDT",       // OKX uses USDT
        _ => "BTC-USD",            // Default fallback
    }
}

/// Get the last update for a specific data type by checking today's time file
/// Files are named like: time.DD.MM.YY.bin
async fn get_data_type_status(type_path: &str) -> DataTypeStatus {
    // Get today's date to construct the filename
    let now = Utc::now();
    let day = now.day();
    let month = now.month();
    let year = now.year() % 100; // Get last 2 digits of year
    
    // Try today's file first
    let today_filename = format!("time.{:02}.{:02}.{:02}.bin", day, month, year);
    let today_path = format!("{}/{}", type_path, today_filename);
    
    // Check if today's file exists
    if let Ok(metadata) = fs::metadata(&today_path).await {
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                let timestamp = duration.as_secs();
                return DataTypeStatus {
                    last_update: timestamp,
                    last_update_date: timestamp_to_readable(timestamp),
                };
            }
        }
    }
    
    // If today's file doesn't exist, try yesterday's
    let yesterday = now - chrono::Duration::days(1);
    let yesterday_filename = format!(
        "time.{:02}.{:02}.{:02}.bin", 
        yesterday.day(), 
        yesterday.month(), 
        yesterday.year() % 100
    );
    let yesterday_path = format!("{}/{}", type_path, yesterday_filename);
    
    if let Ok(metadata) = fs::metadata(&yesterday_path).await {
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                let timestamp = duration.as_secs();
                return DataTypeStatus {
                    last_update: timestamp,
                    last_update_date: timestamp_to_readable(timestamp),
                };
            }
        }
    }
    
    // If neither exists, try the day before (for weekends/holidays)
    let two_days_ago = now - chrono::Duration::days(2);
    let two_days_filename = format!(
        "time.{:02}.{:02}.{:02}.bin", 
        two_days_ago.day(), 
        two_days_ago.month(), 
        two_days_ago.year() % 100
    );
    let two_days_path = format!("{}/{}", type_path, two_days_filename);
    
    if let Ok(metadata) = fs::metadata(&two_days_path).await {
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                let timestamp = duration.as_secs();
                return DataTypeStatus {
                    last_update: timestamp,
                    last_update_date: timestamp_to_readable(timestamp),
                };
            }
        }
    }
    
    // No recent files found
    DataTypeStatus {
        last_update: 0,
        last_update_date: "Never".to_string(),
    }
}

/// Get the last update for an exchange using hardcoded BTC symbols
async fn get_exchange_quick_status(exchange: String) -> ExchangeStatus {
    let base_path = "/mnt/md/data";
    
    // Get the hardcoded BTC symbol for this exchange
    let btc_symbol = get_btc_symbol_for_exchange(&exchange);
    let symbol_path = format!("{}/{}/{}", base_path, exchange, btc_symbol);
    
    // Check if the symbol directory exists
    let symbol_path = std::path::Path::new(&symbol_path);
    if !symbol_path.exists() {
        // Symbol doesn't exist, return empty status
        return ExchangeStatus {
            exchange,
            last_update: 0,
            last_update_date: "Never".to_string(),
            md: DataTypeStatus {
                last_update: 0,
                last_update_date: "Never".to_string(),
            },
            trades: DataTypeStatus {
                last_update: 0,
                last_update_date: "Never".to_string(),
            },
            symbol_checked: format!("{} (not found)", btc_symbol),
        };
    }
    
    // Check MD directory with specific time file
    let md_path = symbol_path.join("MD");
    let md_status = if md_path.exists() {
        get_data_type_status(md_path.to_str().unwrap_or("")).await
    } else {
        DataTypeStatus {
            last_update: 0,
            last_update_date: "Never".to_string(),
        }
    };
    
    // Check TRADES directory with specific time file
    let trades_path = symbol_path.join("TRADES");
    let trades_status = if trades_path.exists() {
        get_data_type_status(trades_path.to_str().unwrap_or("")).await
    } else {
        DataTypeStatus {
            last_update: 0,
            last_update_date: "Never".to_string(),
        }
    };
    
    // Get the most recent update timestamp across both data types
    let last_update = std::cmp::max(md_status.last_update, trades_status.last_update);
    let last_update_date = if last_update > 0 {
        timestamp_to_readable(last_update)
    } else {
        "Never".to_string()
    };
    
    ExchangeStatus {
        exchange,
        last_update,
        last_update_date,
        md: md_status,
        trades: trades_status,
        symbol_checked: btc_symbol.to_string(),
    }
}

/// Handler for the /api/status endpoint - uses hardcoded BTC symbols and exact date files
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
    
    // Hardcode the exchanges we care about (faster than scanning)
    let exchanges = vec![
        "binance".to_string(),
        "coinbase".to_string(),
        "kraken".to_string(),
        "bitfinex".to_string(),
        "okx".to_string(),
    ];
    
    // Process all exchanges in parallel - each one only checks TWO specific files!
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