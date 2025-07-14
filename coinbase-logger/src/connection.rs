use chrono::Local;
use futures_util::{future::try_join_all, SinkExt, StreamExt};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::time::{interval, sleep};
use tokio_tungstenite::{connect_async_with_config, tungstenite::Message};

use crate::data_types::TickerData;
use crate::file_handlers::{open_file, FileHandles, FILE_BUFFER_SIZE};
use crate::websocket::create_websocket_config;
use crate::Result;

pub const BUFFER_FLUSH_INTERVAL: Duration = Duration::from_secs(5);
pub const MAX_BUFFER_SIZE: usize = 10000;
pub const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(60);

pub struct ConnectionHandler {
    pub connection_id: usize,
    pub symbols: Vec<String>,
    pub buffer: BTreeMap<(u64, String), TickerData>,
    pub file_handles: HashMap<String, FileHandles>,
    pub reconnect_delay: Duration,
}

impl ConnectionHandler {
    pub async fn new(connection_id: usize, symbols: Vec<String>) -> Result<Self> {
        let mut file_handles = HashMap::new();
        let date = Local::now().format("%d.%m.%y").to_string();

        for symbol in &symbols {
            let base_path = format!("/usr/src/app/data/{}/MD", symbol);

            match Self::create_file_handles_for_symbol(&base_path, &date).await {
                Ok(handles) => {
                    file_handles.insert(symbol.clone(), handles);
                }
                Err(e) => {
                    eprintln!(
                        "Connection {}: Failed to create file handles for {}: {}",
                        connection_id, symbol, e
                    );

                    for (sym, handles) in file_handles {
                        if let Err(close_err) = handles.close().await {
                            eprintln!("Connection {}: Error closing file handles for {} during cleanup: {}", connection_id, sym, close_err);
                        }
                    }

                    return Err(e);
                }
            }
        }

        Ok(Self {
            connection_id,
            symbols,
            buffer: BTreeMap::new(),
            file_handles,
            reconnect_delay: Duration::from_secs(1),
        })
    }

    pub async fn create_file_handles_for_symbol(
        base_path: &str,
        date: &str,
    ) -> Result<FileHandles> {
        tokio::fs::create_dir_all(&base_path).await?;

        let handles = FileHandles {
            time_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{}/time.{}.bin", base_path, date)).await?,
            ),
            nanos_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{}/nanos.{}.bin", base_path, date)).await?,
            ),
            price_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{}/price.{}.bin", base_path, date)).await?,
            ),
            volume_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{}/volume.{}.bin", base_path, date)).await?,
            ),
            side_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{}/side.{}.bin", base_path, date)).await?,
            ),
            best_bid_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{}/best_bid.{}.bin", base_path, date)).await?,
            ),
            best_ask_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{}/best_ask.{}.bin", base_path, date)).await?,
            ),
        };

        Ok(handles)
    }

    pub async fn run(&mut self) {
        'outer: loop {
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

            if let Err(e) = self.cleanup().await {
                eprintln!(
                    "Connection {}: Error during cleanup: {}",
                    self.connection_id, e
                );
            }

            sleep(self.reconnect_delay).await;

            self.reconnect_delay = std::cmp::min(self.reconnect_delay * 2, MAX_RECONNECT_DELAY);

            let mut retry_count = 0;
            const MAX_RETRIES: u32 = 3;

            loop {
                match self.recreate_file_handles().await {
                    Ok(()) => break,
                    Err(e) => {
                        retry_count += 1;
                        eprintln!(
                            "Connection {}: Failed to recreate file handles (attempt {}/{}): {}",
                            self.connection_id, retry_count, MAX_RETRIES, e
                        );

                        if retry_count >= MAX_RETRIES {
                            eprintln!("Connection {}: Maximum retries exceeded for file handle recreation. Waiting longer before retry.", self.connection_id);
                            sleep(Duration::from_secs(30)).await;
                            continue 'outer;
                        }

                        sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        }
    }

    pub async fn handle_websocket(&mut self) -> Result<()> {
        let config = create_websocket_config();

        let (ws_stream, _) =
            connect_async_with_config("wss://ws-feed.exchange.coinbase.com", Some(config), true)
                .await?;

        println!(
            "Connection {}: Connected to Coinbase WebSocket feed",
            self.connection_id
        );

        self.reconnect_delay = Duration::from_secs(1);

        let (mut write, mut read) = ws_stream.split();

        let subscribe_msg = json!({
            "type": "subscribe",
            "channels": [{
                "name": "ticker",
                "product_ids": self.symbols
            }]
        });
        write.send(Message::Text(subscribe_msg.to_string())).await?;

        let mut flush_interval = interval(BUFFER_FLUSH_INTERVAL);

        loop {
            tokio::select! {
                Some(message) = read.next() => {
                    match message {
                        Ok(msg) if msg.is_text() => {
                            let text = msg.into_text()?;
                            self.process_message(&text).await?;

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

        self.flush_buffer().await?;
        Ok(())
    }

    pub async fn process_message(&mut self, text: &str) -> Result<()> {
        if self.file_handles.is_empty() {
            eprintln!(
                "Connection {}: Ignoring message - no file handles available",
                self.connection_id
            );
            return Ok(());
        }

        let v: serde_json::Value = match serde_json::from_str(text) {
            Ok(val) => val,
            Err(e) => {
                eprintln!(
                    "Connection {}: Failed to parse JSON: {}",
                    self.connection_id, e
                );
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

    pub async fn flush_buffer(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        if self.file_handles.is_empty() {
            eprintln!(
                "Connection {}: Cannot flush buffer - no file handles available",
                self.connection_id
            );
            self.buffer.clear();
            return Ok(());
        }

        println!(
            "Connection {}: Flushing {} messages",
            self.connection_id,
            self.buffer.len()
        );

        for ((_, symbol), data) in self.buffer.iter() {
            if let Some(handles) = self.file_handles.get_mut(symbol) {
                let time_bytes = data.timestamp_secs.to_le_bytes();
                let nanos_bytes = data.timestamp_nanos.to_le_bytes();
                let price_bytes = data.price.to_le_bytes();
                let volume_bytes = data.volume.to_le_bytes();
                let side_bytes = [data.side, 0, 0, 0];
                let best_bid_bytes = data.best_bid.to_le_bytes();
                let best_ask_bytes = data.best_ask.to_le_bytes();

                let write_futures = vec![
                    handles.time_file.write_all(&time_bytes),
                    handles.nanos_file.write_all(&nanos_bytes),
                    handles.price_file.write_all(&price_bytes),
                    handles.volume_file.write_all(&volume_bytes),
                    handles.side_file.write_all(&side_bytes),
                    handles.best_bid_file.write_all(&best_bid_bytes),
                    handles.best_ask_file.write_all(&best_ask_bytes),
                ];

                try_join_all(write_futures).await?;
            }
        }

        let flush_futures: Vec<_> = self
            .file_handles
            .values_mut()
            .map(|handles| handles.flush_all())
            .collect();

        try_join_all(flush_futures).await?;

        self.buffer.clear();
        Ok(())
    }

    pub async fn cleanup(&mut self) -> Result<()> {
        if let Err(e) = self.flush_buffer().await {
            eprintln!(
                "Connection {}: Error flushing buffer during cleanup: {}",
                self.connection_id, e
            );
        }

        let symbols: Vec<String> = self.file_handles.keys().cloned().collect();
        for symbol in symbols {
            if let Some(handles) = self.file_handles.remove(&symbol) {
                if let Err(e) = handles.close().await {
                    eprintln!(
                        "Connection {}: Error closing file handles for {}: {}",
                        self.connection_id, symbol, e
                    );
                }
            }
        }

        self.file_handles.clear();

        Ok(())
    }

    pub async fn recreate_file_handles(&mut self) -> Result<()> {
        let date = Local::now().format("%d.%m.%y").to_string();
        let mut new_file_handles = HashMap::new();

        for symbol in &self.symbols {
            let base_path = format!("/usr/src/app/data/{}/MD", symbol);

            match Self::create_file_handles_for_symbol(&base_path, &date).await {
                Ok(handles) => {
                    new_file_handles.insert(symbol.clone(), handles);
                }
                Err(e) => {
                    eprintln!(
                        "Connection {}: Failed to recreate file handles for {}: {}",
                        self.connection_id, symbol, e
                    );

                    for (sym, handles) in new_file_handles {
                        if let Err(close_err) = handles.close().await {
                            eprintln!("Connection {}: Error closing file handles for {} during cleanup: {}", self.connection_id, sym, close_err);
                        }
                    }

                    return Err(e);
                }
            }
        }

        self.file_handles = new_file_handles;
        Ok(())
    }

    pub fn update_reconnect_delay(&mut self) {
        self.reconnect_delay = std::cmp::min(self.reconnect_delay * 2, MAX_RECONNECT_DELAY);
    }

    pub fn reset_reconnect_delay(&mut self) {
        self.reconnect_delay = Duration::from_secs(1);
    }
}
