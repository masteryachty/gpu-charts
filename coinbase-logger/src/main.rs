use base64::{engine::general_purpose, Engine as _};
use chrono::Local;
use futures_util::{SinkExt, StreamExt};
use http::{HeaderValue, Request};
use rand::Rng;
use serde_json::json;
use std::{error::Error, time::Duration};
use tokio::io::AsyncWriteExt;
use tokio_tungstenite::connect_async_with_config;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::WebSocketConfig, tungstenite::Message,
};
// use url::Url;

/// Use a multi-threaded Tokio runtime with 4 worker threads.
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Box<dyn Error>> {
    // First, connect to Coinbase and subscribe to the "status" channel to fetch all products.
    let products = get_all_products().await?;
    // let products = ["BTC-USDT".to_string(), "ETH-USDT".to_string()];
    println!("Found {} products", products.len());

    // Split the products evenly into 4 groups.
    let chunk_size = (products.len() + 3) / 4; // round up
    let groups: Vec<Vec<String>> = products
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect();

    // // For each group, spawn a task that launches logging tasks for each product.
    for group in groups {
        tokio::spawn(async move {
            for product in group {
                // Spawn a separate task for each product so that each logger runs concurrently.
                tokio::spawn(async move {
                    if let Err(e) = handle_symbol(&product).await {
                        eprintln!("Error handling {}: {}", product, e);
                    }
                });
            }
        });
    }

    // Prevent the main task from exiting.
    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await;
    }
}

/// Connects to Coinbase’s WebSocket feed and subscribes to the "status" channel,
/// waiting for a status message that returns the available products.
/// (This function assumes that the status message has a "products" field that is an array.)
async fn get_all_products() -> Result<Vec<String>, Box<dyn Error>> {
    // Build the connection request.
    println!("trying to Coinbase WebSocket feed for status");

    // let request = Request::builder()
    //     .uri("wss://ws-feed.exchange.coinbase.com")
    //     // .header("Sec-WebSocket-Extensions", HeaderValue::from_static("permessage-deflate"))
    //     .body(())?;
    let (ws_stream, _) = connect_async("wss://ws-feed.exchange.coinbase.com").await?;
    println!("Connected to Coinbase WebSocket feed for status");

    let (mut write, mut read) = ws_stream.split();

    // Subscribe to the "status" channel.
    let subscribe_msg = json!({
        "type": "subscribe",
        "channels": [{
            "name": "status"
        }]
    });
    write.send(Message::Text(subscribe_msg.to_string())).await?;

    // Wait for a message of type "status" that contains a "products" field.
    while let Some(message) = read.next().await {
        match message {
            Ok(msg) if msg.is_text() => {
                let text = msg.into_text()?;
                let v: serde_json::Value = serde_json::from_str(&text)?;
                // println!("status {:?}", v);

                if v.get("type") == Some(&serde_json::Value::String("status".to_string())) {
                    if let Some(products_array) = v.get("products").and_then(|p| p.as_array()) {
                        let products = products_array
                            .iter()
                            .filter_map(|p| {
                                if p.get("status").and_then(|s| s.as_str()) == Some("online") {
                                    p.get("id")
                                        .and_then(|id| id.as_str())
                                        .map(|s| s.to_string())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        return Ok(products);
                    } else {
                        eprintln!("Status message did not contain a valid 'products' field.");
                    }
                }
            }
            Ok(_) => continue,
            Err(e) => eprintln!("Error reading status message: {}", e),
        }
    }
    Err("No status message received from Coinbase".into())
}

/// For a given symbol, creates the directory structure and repeatedly connects to Coinbase's
/// WebSocket feed to subscribe to ticker data.
async fn handle_symbol(symbol: &str) -> Result<(), Box<dyn Error>> {
    // Create a directory structure: "./data/{symbol}/MD"
    let base_path = format!("/mnt/md/data/{}/MD", symbol);
    tokio::fs::create_dir_all(&base_path).await?;
    let date = Local::now().format("%d.%m.%y").to_string();

    // Define file paths.
    let time_file_path = format!("{}/time.{}.bin", base_path, date);
    let price_file_path = format!("{}/price.{}.bin", base_path, date);
    let volume_file_path = format!("{}/volume.{}.bin", base_path, date);
    let side_file_path = format!("{}/side.{}.bin", base_path, date);
    let best_bid_file_path = format!("{}/best_bid.{}.bin", base_path, date);
    let best_ask_file_path = format!("{}/best_ask.{}.bin", base_path, date);

    println!(
        "Logging market data for {} into directory {}",
        symbol, base_path
    );

    // Reconnect loop
    loop {
        println!("Connecting to Coinbase WebSocket feed for {}...", symbol);
        match run_websocket(
            symbol,
            &time_file_path,
            &price_file_path,
            &volume_file_path,
            &side_file_path,
            &best_bid_file_path,
            &best_ask_file_path,
        )
        .await
        {
            Ok(_) => {
                eprintln!(
                    "{}: WebSocket stream ended gracefully. Reconnecting in 5 seconds...",
                    symbol
                );
            }
            Err(e) => {
                eprintln!(
                    "{}: Error in WebSocket connection: {}. Reconnecting in 5 seconds...",
                    symbol, e
                );
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

/// Connects to Coinbase's WebSocket, subscribes to the ticker channel for the provided symbol,
/// and logs received ticker data to separate files.
async fn run_websocket(
    symbol: &str,
    time_file_path: &str,
    price_file_path: &str,
    volume_file_path: &str,
    side_file_path: &str,
    best_bid_file_path: &str,
    best_ask_file_path: &str,
) -> Result<(), Box<dyn Error>> {
    // let random_bytes: [u8; 16] = rand::thread_rng().gen();
    // let sec_websocket_key = general_purpose::STANDARD.encode(&random_bytes);

    // let request = Request::builder()
    //     .uri("wss://ws-feed.exchange.coinbase.com")
    //     .header("Host", "ws-feed.exchange.coinbase.com")
    //     .header("Connection", "Upgrade")
    //     .header("Upgrade", "websocket")
    //     .header("Sec-WebSocket-Version", "13")
    //     .header(
    //         "Sec-WebSocket-Extensions",
    //         HeaderValue::from_static("permessage-deflate"),
    //     )
    //     .header("Sec-WebSocket-Key", sec_websocket_key)
    //     .body(())?;
    let config = WebSocketConfig {
        // Increase the maximum size of a complete message to 64 MB
        max_message_size: Some(64 << 20),
        // Increase the maximum frame size if needed
        max_frame_size: Some(16 << 20),
        // Optionally configure the send queue, etc.
        max_send_queue: Some(100),
        write_buffer_size: 8191, // 8 KiB is typically sufficient for small messages
        max_write_buffer_size: 8192,
        accept_unmasked_frames: false,
    };

    let (ws_stream, response) =
        connect_async_with_config("wss://ws-feed.exchange.coinbase.com", Some(config), true)
            .await?;

    println!("{}: Connected to Coinbase WebSocket feed", symbol);

    let (mut write, mut read) = ws_stream.split();

    // Subscribe to the ticker channel for the symbol.
    let subscribe_msg = json!({
        "type": "subscribe",
        "channels": [{
            "name": "ticker",
            "product_ids": [symbol]
        }]
    });
    write.send(Message::Text(subscribe_msg.to_string())).await?;

    // Open files in append mode.
    let mut time_file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(time_file_path)
        .await?;
    let mut price_file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(price_file_path)
        .await?;
    let mut volume_file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(volume_file_path)
        .await?;
    let mut side_file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(side_file_path)
        .await?;
    let mut best_bid_file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(best_bid_file_path)
        .await?;
    let mut best_ask_file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(best_ask_file_path)
        .await?;

    // Process incoming messages.
    while let Some(message) = read.next().await {
        match message {
            Ok(msg) => {
                println!("{}: Received message: {:?}", symbol, msg);
                if msg.is_text() {
                    let text = msg.into_text()?;
                    let v: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(val) => val,
                        Err(e) => {
                            eprintln!("{}: Failed to parse JSON: {}", symbol, e);
                            continue;
                        }
                    };

                    if v.get("type") == Some(&serde_json::Value::String("ticker".to_string())) {
                        let time_str = v.get("time").and_then(|v| v.as_str());
                        let price_str = v.get("price").and_then(|v| v.as_str());
                        let last_size_str = v.get("last_size").and_then(|v| v.as_str());
                        let side_str = v.get("side").and_then(|v| v.as_str());
                        let best_bid_str = v.get("best_bid").and_then(|v| v.as_str());
                        let best_ask_str = v.get("best_ask").and_then(|v| v.as_str());

                        if let (
                            Some(time_str),
                            Some(price_str),
                            Some(last_size_str),
                            Some(side_str),
                            Some(best_bid_str),
                            Some(best_ask_str),
                        ) = (
                            time_str,
                            price_str,
                            last_size_str,
                            side_str,
                            best_bid_str,
                            best_ask_str,
                        ) {
                            match chrono::DateTime::parse_from_rfc3339(time_str) {
                                Ok(dt) => {
                                    let timestamp = dt.timestamp() as u32;
                                    let nanos = dt.timestamp_subsec_nanos() as u32;
                                    println!("timestamp, nans {}, {}", timestamp, nanos);
                                    if let Ok(price) = price_str.parse::<f32>() {
                                        if let Ok(volume) = last_size_str.parse::<f32>() {
                                            if let Ok(best_bid) = best_bid_str.parse::<f32>() {
                                                if let Ok(best_ask) = best_ask_str.parse::<f32>() {
                                                    let side = match side_str {
                                                        "buy" => 1u8,
                                                        "sell" => 0u8,
                                                        other => {
                                                            eprintln!(
                                                                "{}: Unrecognized side value: {}",
                                                                symbol, other
                                                            );
                                                            continue;
                                                        }
                                                    };

                                                    // Convert values to little–endian bytes.
                                                    let time_bytes = timestamp.to_le_bytes();
                                                    let price_bytes = price.to_le_bytes();
                                                    let volume_bytes = volume.to_le_bytes();
                                                    let best_bid_bytes = best_bid.to_le_bytes();
                                                    let best_ask_bytes = best_ask.to_le_bytes();
                                                    let side_bytes = [side];

                                                    if let Err(e) =
                                                        time_file.write_all(&time_bytes).await
                                                    {
                                                        eprintln!(
                                                            "{}: Error writing time: {}",
                                                            symbol, e
                                                        );
                                                    }
                                                    if let Err(e) =
                                                        price_file.write_all(&price_bytes).await
                                                    {
                                                        eprintln!(
                                                            "{}: Error writing price: {}",
                                                            symbol, e
                                                        );
                                                    }
                                                    if let Err(e) =
                                                        volume_file.write_all(&volume_bytes).await
                                                    {
                                                        eprintln!(
                                                            "{}: Error writing volume: {}",
                                                            symbol, e
                                                        );
                                                    }
                                                    if let Err(e) =
                                                        side_file.write_all(&side_bytes).await
                                                    {
                                                        eprintln!(
                                                            "{}: Error writing side: {}",
                                                            symbol, e
                                                        );
                                                    }
                                                    if let Err(e) = best_bid_file
                                                        .write_all(&best_bid_bytes)
                                                        .await
                                                    {
                                                        eprintln!(
                                                            "{}: Error writing best_bid: {}",
                                                            symbol, e
                                                        );
                                                    }
                                                    if let Err(e) = best_ask_file
                                                        .write_all(&best_ask_bytes)
                                                        .await
                                                    {
                                                        eprintln!(
                                                            "{}: Error writing best_ask: {}",
                                                            symbol, e
                                                        );
                                                    }

                                                    println!("{}: Logged record: time={} price={} volume={} side={} best_bid={} best_ask={}",
                                                        symbol, timestamp, price, volume, side, best_bid, best_ask);
                                                } else {
                                                    eprintln!(
                                                        "{}: Failed to parse best_ask as f32: {}",
                                                        symbol, best_ask_str
                                                    );
                                                }
                                            } else {
                                                eprintln!(
                                                    "{}: Failed to parse best_bid as f32: {}",
                                                    symbol, best_bid_str
                                                );
                                            }
                                        } else {
                                            eprintln!(
                                                "{}: Failed to parse last_size as f32: {}",
                                                symbol, last_size_str
                                            );
                                        }
                                    } else {
                                        eprintln!(
                                            "{}: Failed to parse price as f32: {}",
                                            symbol, price_str
                                        );
                                    }
                                }
                                Err(e) => {
                                    eprintln!("{}: Error parsing time {}: {}", symbol, time_str, e);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("{}: WebSocket error: {}", symbol, e);
                break;
            }
        }
    }

    Ok(())
}
