use chrono::Local;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::time::{interval, sleep};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{protocol::WebSocketConfig, Message},
};

type Error = Box<dyn std::error::Error + Send + Sync>;

const CONNECTIONS_COUNT: usize = 10;
const BUFFER_FLUSH_INTERVAL: Duration = Duration::from_secs(5); // Increased to 5 seconds
const MAX_BUFFER_SIZE: usize = 10000; // Flush if buffer gets this large
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(60);
const FILE_BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer for file writes

#[derive(Clone)]
struct TickerData {
    timestamp_secs: u32,
    timestamp_nanos: u32,
    price: f32,
    volume: f32,
    side: u8,
    best_bid: f32,
    best_ask: f32,
}

struct FileHandles {
    time_file: BufWriter<File>,
    nanos_file: BufWriter<File>,
    price_file: BufWriter<File>,
    volume_file: BufWriter<File>,
    side_file: BufWriter<File>,
    best_bid_file: BufWriter<File>,
    best_ask_file: BufWriter<File>,
}

struct ConnectionHandler {
    connection_id: usize,
    symbols: Vec<String>,
    buffer: BTreeMap<(u64, String), TickerData>, // (timestamp_nanos, symbol) for sorting
    file_handles: HashMap<String, FileHandles>,
    reconnect_delay: Duration,
}

impl ConnectionHandler {
    async fn new(connection_id: usize, symbols: Vec<String>) -> Result<Self, Error> {
        let mut file_handles = HashMap::new();
        let date = Local::now().format("%d.%m.%y").to_string();

        // Create file handles for each symbol
        for symbol in &symbols {
            let base_path = format!("/usr/src/app/data/{}/MD", symbol);
            tokio::fs::create_dir_all(&base_path).await?;

            let handles = FileHandles {
                time_file: BufWriter::with_capacity(FILE_BUFFER_SIZE, open_file(&format!("{}/time.{}.bin", base_path, date)).await?),
                nanos_file: BufWriter::with_capacity(FILE_BUFFER_SIZE, open_file(&format!("{}/nanos.{}.bin", base_path, date)).await?),
                price_file: BufWriter::with_capacity(FILE_BUFFER_SIZE, open_file(&format!("{}/price.{}.bin", base_path, date)).await?),
                volume_file: BufWriter::with_capacity(FILE_BUFFER_SIZE, open_file(&format!("{}/volume.{}.bin", base_path, date)).await?),
                side_file: BufWriter::with_capacity(FILE_BUFFER_SIZE, open_file(&format!("{}/side.{}.bin", base_path, date)).await?),
                best_bid_file: BufWriter::with_capacity(FILE_BUFFER_SIZE, open_file(&format!("{}/best_bid.{}.bin", base_path, date)).await?),
                best_ask_file: BufWriter::with_capacity(FILE_BUFFER_SIZE, open_file(&format!("{}/best_ask.{}.bin", base_path, date)).await?),
            };

            file_handles.insert(symbol.clone(), handles);
        }

        Ok(Self {
            connection_id,
            symbols,
            buffer: BTreeMap::new(),
            file_handles,
            reconnect_delay: Duration::from_secs(1),
        })
    }

    async fn run(&mut self) {
        loop {
            println!(
                "Connection {}: Connecting to Coinbase WebSocket for {} symbols...",
                self.connection_id,
                self.symbols.len()
            );

            match self.handle_websocket().await {
                Ok(_) => {
                    eprintln!(
                        "Connection {}: WebSocket stream ended gracefully. Reconnecting in {:?}...",
                        self.connection_id, self.reconnect_delay
                    );
                }
                Err(e) => {
                    eprintln!(
                        "Connection {}: Error in WebSocket connection: {}. Reconnecting in {:?}...",
                        self.connection_id, e, self.reconnect_delay
                    );
                }
            }

            sleep(self.reconnect_delay).await;

            // Exponential backoff
            self.reconnect_delay = std::cmp::min(self.reconnect_delay * 2, MAX_RECONNECT_DELAY);
        }
    }

    async fn handle_websocket(&mut self) -> Result<(), Error> {
        let config = WebSocketConfig {
            max_message_size: Some(64 << 20),
            max_frame_size: Some(16 << 20),
            write_buffer_size: 256 * 1024, // Increased from 8KB to 256KB
            max_write_buffer_size: 512 * 1024, // Increased to 512KB
            accept_unmasked_frames: false,
            ..Default::default()
        };

        let (ws_stream, _) =
            connect_async_with_config("wss://ws-feed.exchange.coinbase.com", Some(config), true)
                .await?;

        println!(
            "Connection {}: Connected to Coinbase WebSocket feed",
            self.connection_id
        );

        // Reset reconnect delay on successful connection
        self.reconnect_delay = Duration::from_secs(1);

        let (mut write, mut read) = ws_stream.split();

        // Subscribe to ticker channel for multiple symbols
        let subscribe_msg = json!({
            "type": "subscribe",
            "channels": [{
                "name": "ticker",
                "product_ids": self.symbols
            }]
        });
        write.send(Message::Text(subscribe_msg.to_string())).await?;

        // Set up flush interval
        let mut flush_interval = interval(BUFFER_FLUSH_INTERVAL);

        loop {
            tokio::select! {
                Some(message) = read.next() => {
                    match message {
                        Ok(msg) if msg.is_text() => {
                            let text = msg.into_text()?;
                            self.process_message(&text).await?;
                            
                            // Smart flushing: flush if buffer is getting large
                            if self.buffer.len() >= MAX_BUFFER_SIZE {
                                self.flush_buffer().await?;
                            }
                        }
                        Ok(_) => continue,
                        Err(e) => {
                            eprintln!("Connection {}: WebSocket error: {}", self.connection_id, e);
                            break;
                        }
                    }
                }
                _ = flush_interval.tick() => {
                    self.flush_buffer().await?;
                }
            }
        }

        // Flush any remaining messages
        self.flush_buffer().await?;
        Ok(())
    }

    async fn process_message(&mut self, text: &str) -> Result<(), Error> {
        let v: serde_json::Value = match serde_json::from_str(text) {
            Ok(val) => val,
            Err(e) => {
                eprintln!("Connection {}: Failed to parse JSON: {}", self.connection_id, e);
                return Ok(());
            }
        };

        if v.get("type") == Some(&json!("ticker")) {
            if let (
                Some(product_id),
                Some(time_str),
                Some(price_str),
                Some(last_size_str),
                Some(side_str),
                Some(best_bid_str),
                Some(best_ask_str),
            ) = (
                v.get("product_id").and_then(|v| v.as_str()),
                v.get("time").and_then(|v| v.as_str()),
                v.get("price").and_then(|v| v.as_str()),
                v.get("last_size").and_then(|v| v.as_str()),
                v.get("side").and_then(|v| v.as_str()),
                v.get("best_bid").and_then(|v| v.as_str()),
                v.get("best_ask").and_then(|v| v.as_str()),
            ) {
                // Parse timestamp
                let dt = match chrono::DateTime::parse_from_rfc3339(time_str) {
                    Ok(dt) => dt,
                    Err(e) => {
                        eprintln!(
                            "Connection {}: Error parsing time {}: {}",
                            self.connection_id, time_str, e
                        );
                        return Ok(());
                    }
                };

                let timestamp_secs = dt.timestamp() as u32;
                let timestamp_nanos = dt.timestamp_subsec_nanos();

                // Parse numeric values
                let price = price_str.parse::<f32>().ok();
                let volume = last_size_str.parse::<f32>().ok();
                let best_bid = best_bid_str.parse::<f32>().ok();
                let best_ask = best_ask_str.parse::<f32>().ok();

                if let (Some(price), Some(volume), Some(best_bid), Some(best_ask)) =
                    (price, volume, best_bid, best_ask)
                {
                    let side = match side_str {
                        "buy" => 1u8,
                        "sell" => 0u8,
                        _ => return Ok(()),
                    };

                    let ticker_data = TickerData {
                        timestamp_secs,
                        timestamp_nanos,
                        price,
                        volume,
                        side,
                        best_bid,
                        best_ask,
                    };

                    // Create unique key for sorting (full timestamp in nanos + symbol)
                    let key = (
                        (timestamp_secs as u64) * 1_000_000_000 + (timestamp_nanos as u64),
                        product_id.to_string(),
                    );

                    self.buffer.insert(key, ticker_data);
                }
            }
        }

        Ok(())
    }

    async fn flush_buffer(&mut self) -> Result<(), Error> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        println!(
            "Connection {}: Flushing {} messages",
            self.connection_id,
            self.buffer.len()
        );

        // Process messages in sorted order
        for ((_, symbol), data) in self.buffer.iter() {
            if let Some(handles) = self.file_handles.get_mut(symbol) {
                // Write all data fields
                handles.time_file.write_all(&data.timestamp_secs.to_le_bytes()).await?;
                handles.nanos_file.write_all(&data.timestamp_nanos.to_le_bytes()).await?;
                handles.price_file.write_all(&data.price.to_le_bytes()).await?;
                handles.volume_file.write_all(&data.volume.to_le_bytes()).await?;
                handles.side_file.write_all(&[data.side, 0, 0, 0]).await?; // Pad to 4 bytes
                handles.best_bid_file.write_all(&data.best_bid.to_le_bytes()).await?;
                handles.best_ask_file.write_all(&data.best_ask.to_le_bytes()).await?;
            }
        }

        // Flush all buffered writers for this symbol
        for handles in self.file_handles.values_mut() {
            handles.time_file.flush().await?;
            handles.nanos_file.flush().await?;
            handles.price_file.flush().await?;
            handles.volume_file.flush().await?;
            handles.side_file.flush().await?;
            handles.best_bid_file.flush().await?;
            handles.best_ask_file.flush().await?;
        }

        self.buffer.clear();
        Ok(())
    }
}

async fn open_file(path: &str) -> Result<File, Error> {
    Ok(tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?)
}

async fn get_all_products() -> Result<Vec<String>, Error> {
    println!("Fetching all available products from Coinbase...");

    let (ws_stream, _) = connect_async_with_config(
        "wss://ws-feed.exchange.coinbase.com",
        Some(WebSocketConfig::default()),
        true,
    )
    .await?;

    let (mut write, mut read) = ws_stream.split();

    // Subscribe to status channel
    let subscribe_msg = json!({
        "type": "subscribe",
        "channels": [{
            "name": "status"
        }]
    });
    write.send(Message::Text(subscribe_msg.to_string())).await?;

    // Wait for status message
    while let Some(message) = read.next().await {
        if let Ok(msg) = message {
            if msg.is_text() {
                let text = msg.into_text()?;
                let v: serde_json::Value = serde_json::from_str(&text)?;

                if v.get("type") == Some(&json!("status")) {
                    if let Some(products_array) = v.get("products").and_then(|p| p.as_array()) {
                        let products = products_array
                            .iter()
                            .filter_map(|p| {
                                if p.get("status").and_then(|s| s.as_str()) == Some("online") {
                                    p.get("id").and_then(|id| id.as_str()).map(String::from)
                                } else {
                                    None
                                }
                            })
                            .collect();
                        return Ok(products);
                    }
                }
            }
        }
    }

    Err("No status message received from Coinbase".into())
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Error> {
    // Fetch all available products
    let products = get_all_products().await?;
    println!("Found {} products", products.len());

    // Calculate symbols per connection
    let symbols_per_connection = (products.len() + CONNECTIONS_COUNT - 1) / CONNECTIONS_COUNT;

    // Create connection handlers
    let mut tasks = vec![];

    for i in 0..CONNECTIONS_COUNT {
        let start_idx = i * symbols_per_connection;
        let end_idx = std::cmp::min((i + 1) * symbols_per_connection, products.len());

        if start_idx >= products.len() {
            break;
        }

        let connection_symbols = products[start_idx..end_idx].to_vec();
        
        println!(
            "Connection {}: Handling {} symbols",
            i,
            connection_symbols.len()
        );

        let task = tokio::spawn(async move {
            let mut handler = match ConnectionHandler::new(i, connection_symbols).await {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Failed to create connection handler {}: {}", i, e);
                    return;
                }
            };

            handler.run().await;
        });

        tasks.push(task);

        // No rate limiting - launch connections concurrently
    }

    // Wait for all tasks (they run forever)
    for task in tasks {
        let _ = task.await;
    }

    Ok(())
}