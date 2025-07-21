use chrono::Local;
use futures_util::{future::try_join_all, SinkExt, StreamExt};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::time::{interval, sleep};
use tokio_tungstenite::{connect_async_with_config, tungstenite::Message};

use crate::data_types::{MarketTradeData, TickerData, TickerTradeData, uuid_to_bytes};
use crate::file_handlers::{
    open_file, FileHandles, MarketTradeFileHandles, TradeFileHandles, FILE_BUFFER_SIZE,
};
use crate::simple_analytics::AnalyticsManager;
use crate::websocket::create_websocket_config;
use crate::Result;

pub const BUFFER_FLUSH_INTERVAL: Duration = Duration::from_secs(5);
pub const MAX_BUFFER_SIZE: usize = 10000;
pub const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(60);

pub struct ConnectionHandler {
    pub connection_id: usize,
    pub symbols: Vec<String>,
    pub buffer: BTreeMap<(u64, String), TickerData>,
    pub trade_buffer: BTreeMap<(u64, String), TickerTradeData>,
    pub market_trades_buffer: BTreeMap<(u64, String), MarketTradeData>,
    pub file_handles: HashMap<String, FileHandles>,
    pub trade_file_handles: HashMap<String, TradeFileHandles>,
    pub market_trade_file_handles: HashMap<String, MarketTradeFileHandles>,
    pub analytics_manager: AnalyticsManager,
    pub reconnect_delay: Duration,
    pub current_date: String,
}

impl ConnectionHandler {
    pub async fn new(connection_id: usize, symbols: Vec<String>) -> Result<Self> {
        let mut file_handles = HashMap::new();
        let mut trade_file_handles = HashMap::new();
        let mut market_trade_file_handles = HashMap::new();
        let date = Local::now().format("%d.%m.%y").to_string();

        for symbol in &symbols {
            let base_path = format!("/mnt/md/data/{symbol}");

            // Create regular market data file handles
            match Self::create_file_handles_for_symbol(&base_path, &date).await {
                Ok(handles) => {
                    file_handles.insert(symbol.clone(), handles);
                }
                Err(e) => {
                    eprintln!(
                        "Connection {connection_id}: Failed to create file handles for {symbol}: {e}"
                    );

                    // Cleanup already created handles
                    Self::cleanup_all_handles(
                        file_handles,
                        trade_file_handles,
                        market_trade_file_handles,
                        connection_id,
                    )
                    .await;

                    return Err(e);
                }
            }

            // Create trade-specific file handles
            match Self::create_trade_file_handles_for_symbol(&base_path, &date).await {
                Ok(handles) => {
                    trade_file_handles.insert(symbol.clone(), handles);
                }
                Err(e) => {
                    eprintln!(
                        "Connection {connection_id}: Failed to create trade file handles for {symbol}: {e}"
                    );

                    // Cleanup all handles
                    Self::cleanup_all_handles(
                        file_handles,
                        trade_file_handles,
                        market_trade_file_handles,
                        connection_id,
                    )
                    .await;

                    return Err(e);
                }
            }

            // Create market trade file handles
            match Self::create_market_trade_file_handles_for_symbol(&base_path, &date).await {
                Ok(handles) => {
                    market_trade_file_handles.insert(symbol.clone(), handles);
                }
                Err(e) => {
                    eprintln!(
                        "Connection {connection_id}: Failed to create market trade file handles for {symbol}: {e}"
                    );

                    // Cleanup all handles
                    Self::cleanup_all_handles(
                        file_handles,
                        trade_file_handles,
                        market_trade_file_handles,
                        connection_id,
                    )
                    .await;

                    return Err(e);
                }
            }
        }

        // Create analytics manager with 0.1 threshold for large trades (for testing)
        let analytics_manager = AnalyticsManager::new(0.1);

        Ok(Self {
            connection_id,
            symbols,
            buffer: BTreeMap::new(),
            trade_buffer: BTreeMap::new(),
            market_trades_buffer: BTreeMap::new(),
            file_handles,
            trade_file_handles,
            market_trade_file_handles,
            analytics_manager,
            reconnect_delay: Duration::from_secs(1),
            current_date: date,
        })
    }

    pub async fn create_file_handles_for_symbol(
        base_path: &str,
        date: &str,
    ) -> Result<FileHandles> {
        let md_path = format!("{}/MD", base_path);
        tokio::fs::create_dir_all(&md_path).await?;

        let handles = FileHandles {
            time_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{md_path}/time.{date}.bin")).await?,
            ),
            nanos_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{md_path}/nanos.{date}.bin")).await?,
            ),
            price_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{md_path}/price.{date}.bin")).await?,
            ),
            volume_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{md_path}/volume.{date}.bin")).await?,
            ),
            side_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{md_path}/side.{date}.bin")).await?,
            ),
            best_bid_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{md_path}/best_bid.{date}.bin")).await?,
            ),
            best_ask_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{md_path}/best_ask.{date}.bin")).await?,
            ),
        };

        Ok(handles)
    }

    pub async fn create_trade_file_handles_for_symbol(
        base_path: &str,
        date: &str,
    ) -> Result<TradeFileHandles> {
        let trade_path = format!("{}/TICKER_TRADES", base_path);
        tokio::fs::create_dir_all(&trade_path).await?;

        let handles = TradeFileHandles {
            trade_time_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{trade_path}/trade_time.{date}.bin")).await?,
            ),
            trade_nanos_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{trade_path}/trade_nanos.{date}.bin")).await?,
            ),
            trade_price_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{trade_path}/trade_price.{date}.bin")).await?,
            ),
            trade_volume_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{trade_path}/trade_volume.{date}.bin")).await?,
            ),
            trade_side_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{trade_path}/trade_side.{date}.bin")).await?,
            ),
            trade_spread_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{trade_path}/trade_spread.{date}.bin")).await?,
            ),
        };

        Ok(handles)
    }

    pub async fn create_market_trade_file_handles_for_symbol(
        base_path: &str,
        date: &str,
    ) -> Result<MarketTradeFileHandles> {
        let trade_path = format!("{}/TRADES", base_path);
        tokio::fs::create_dir_all(&trade_path).await?;

        let handles = MarketTradeFileHandles {
            trade_id_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{trade_path}/trade_id.{date}.bin")).await?,
            ),
            trade_time_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{trade_path}/trade_time.{date}.bin")).await?,
            ),
            trade_nanos_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{trade_path}/trade_nanos.{date}.bin")).await?,
            ),
            trade_price_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{trade_path}/trade_price.{date}.bin")).await?,
            ),
            trade_size_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{trade_path}/trade_size.{date}.bin")).await?,
            ),
            trade_side_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{trade_path}/trade_side.{date}.bin")).await?,
            ),
            maker_order_id_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{trade_path}/maker_order_id.{date}.bin")).await?,
            ),
            taker_order_id_file: BufWriter::with_capacity(
                FILE_BUFFER_SIZE,
                open_file(&format!("{trade_path}/taker_order_id.{date}.bin")).await?,
            ),
        };

        Ok(handles)
    }

    async fn cleanup_all_handles(
        file_handles: HashMap<String, FileHandles>,
        trade_file_handles: HashMap<String, TradeFileHandles>,
        market_trade_file_handles: HashMap<String, MarketTradeFileHandles>,
        connection_id: usize,
    ) {
        for (sym, handles) in file_handles {
            if let Err(e) = handles.close().await {
                eprintln!(
                    "Connection {connection_id}: Error closing file handles for {sym}: {e}"
                );
            }
        }
        for (sym, handles) in trade_file_handles {
            if let Err(e) = handles.close().await {
                eprintln!(
                    "Connection {connection_id}: Error closing trade file handles for {sym}: {e}"
                );
            }
        }
        for (sym, handles) in market_trade_file_handles {
            if let Err(e) = handles.close().await {
                eprintln!(
                    "Connection {connection_id}: Error closing market trade file handles for {sym}: {e}"
                );
            }
        }
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
            "channels": [
                {
                    "name": "ticker",
                    "product_ids": self.symbols.clone()
                },
                {
                    "name": "matches",
                    "product_ids": self.symbols.clone()
                }
            ]
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
                    // Check if we need to rotate files at midnight
                    if let Err(e) = self.check_and_rotate_files().await {
                        eprintln!("Connection {}: Error during file rotation: {}", self.connection_id, e);
                    }
                    
                    self.flush_buffer().await?;
                    
                    // Generate and log analytics reports
                    let reports = self.analytics_manager.generate_reports();
                    for report in reports {
                        println!("Connection {}: Analytics - {}", 
                            self.connection_id, 
                            report.to_log_string()
                        );
                    }
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

        match v.get("type").and_then(|t| t.as_str()) {
            Some("ticker") => self.process_ticker(&v).await?,
            Some("match") | Some("last_match") => self.process_market_trade(&v).await?,
            Some("subscriptions") => {
                eprintln!("Connection {}: Subscription confirmed", self.connection_id);
            }
            Some("error") => {
                eprintln!(
                    "Connection {}: Error from Coinbase: {:?}",
                    self.connection_id, v
                );
            }
            _ => {
                // Ignore other message types
            }
        }

        Ok(())
    }

    async fn process_ticker(&mut self, v: &serde_json::Value) -> Result<()> {
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

                // Create trade-specific data from ticker
                let trade_data = TickerTradeData::from_ticker(&ticker_data);

                // Validate trade data before storing
                if trade_data.is_valid() {
                    let key = (
                        (timestamp_secs as u64) * 1_000_000_000 + (timestamp_nanos as u64),
                        product_id.to_string(),
                    );

                    self.buffer.insert(key.clone(), ticker_data);
                    self.trade_buffer.insert(key, trade_data);
                } else {
                    eprintln!(
                        "Connection {}: Invalid trade data for {}: price={}, volume={}, side={}, spread={}",
                        self.connection_id, product_id, price, volume, side, best_ask - best_bid
                    );
                }
            }
        }
        Ok(())
    }

    async fn process_market_trade(&mut self, v: &serde_json::Value) -> Result<()> {
        if let (
            Some(product_id),
            Some(trade_id),
            Some(time_str),
            Some(price_str),
            Some(size_str),
            Some(side_str),
        ) = (
            v.get("product_id").and_then(|v| v.as_str()),
            v.get("trade_id").and_then(|v| v.as_u64()),
            v.get("time").and_then(|v| v.as_str()),
            v.get("price").and_then(|v| v.as_str()),
            v.get("size").and_then(|v| v.as_str()),
            v.get("side").and_then(|v| v.as_str()),
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

            let price = match price_str.parse::<f32>() {
                Ok(p) => p,
                Err(_) => return Ok(()),
            };

            let size = match size_str.parse::<f32>() {
                Ok(s) => s,
                Err(_) => return Ok(()),
            };

            let side = match side_str {
                "buy" => 1u8,
                "sell" => 0u8,
                _ => return Ok(()),
            };

            // Handle optional order IDs
            let maker_order_id = match v
                .get("maker_order_id")
                .and_then(|v| v.as_str())
                .and_then(|s| uuid_to_bytes(s).ok())
            {
                Some(bytes) => bytes,
                None => [0u8; 16], // Default if not provided
            };

            let taker_order_id = match v
                .get("taker_order_id")
                .and_then(|v| v.as_str())
                .and_then(|s| uuid_to_bytes(s).ok())
            {
                Some(bytes) => bytes,
                None => [0u8; 16], // Default if not provided
            };

            let market_trade = MarketTradeData {
                trade_id,
                timestamp_secs,
                timestamp_nanos,
                price,
                size,
                side,
                maker_order_id,
                taker_order_id,
            };

            if market_trade.is_valid() {
                let key = (
                    (timestamp_secs as u64) * 1_000_000_000 + (timestamp_nanos as u64),
                    product_id.to_string(),
                );

                // Process trade for analytics
                self.analytics_manager.process_trade(product_id, &market_trade);

                self.market_trades_buffer.insert(key, market_trade);

                // Flush if buffer is getting large
                if self.market_trades_buffer.len() >= MAX_BUFFER_SIZE {
                    self.flush_buffer().await?;
                }
            }
        }

        Ok(())
    }

    pub async fn flush_buffer(&mut self) -> Result<()> {
        if self.buffer.is_empty() && self.trade_buffer.is_empty() && self.market_trades_buffer.is_empty() {
            return Ok(());
        }

        if self.file_handles.is_empty() {
            eprintln!(
                "Connection {}: Cannot flush buffer - no file handles available",
                self.connection_id
            );
            self.buffer.clear();
            self.trade_buffer.clear();
            self.market_trades_buffer.clear();
            return Ok(());
        }

        println!(
            "Connection {}: Flushing {} ticker messages, {} trade messages, and {} market trades",
            self.connection_id,
            self.buffer.len(),
            self.trade_buffer.len(),
            self.market_trades_buffer.len()
        );

        // Write ticker data
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

        // Write trade data
        for ((_, symbol), trade_data) in self.trade_buffer.iter() {
            if let Some(handles) = self.trade_file_handles.get_mut(symbol) {
                let time_bytes = trade_data.timestamp_secs.to_le_bytes();
                let nanos_bytes = trade_data.timestamp_nanos.to_le_bytes();
                let price_bytes = trade_data.trade_price.to_le_bytes();
                let volume_bytes = trade_data.trade_volume.to_le_bytes();
                let side_bytes = [trade_data.trade_side, 0, 0, 0];
                let spread_bytes = trade_data.spread.to_le_bytes();

                let write_futures = vec![
                    handles.trade_time_file.write_all(&time_bytes),
                    handles.trade_nanos_file.write_all(&nanos_bytes),
                    handles.trade_price_file.write_all(&price_bytes),
                    handles.trade_volume_file.write_all(&volume_bytes),
                    handles.trade_side_file.write_all(&side_bytes),
                    handles.trade_spread_file.write_all(&spread_bytes),
                ];

                try_join_all(write_futures).await?;
            }
        }

        // Flush ticker files
        let flush_futures: Vec<_> = self
            .file_handles
            .values_mut()
            .map(|handles| handles.flush_all())
            .collect();

        try_join_all(flush_futures).await?;

        // Write market trade data
        for ((_, symbol), market_trade) in self.market_trades_buffer.iter() {
            if let Some(handles) = self.market_trade_file_handles.get_mut(symbol) {
                let trade_id_bytes = market_trade.trade_id.to_le_bytes();
                let time_bytes = market_trade.timestamp_secs.to_le_bytes();
                let nanos_bytes = market_trade.timestamp_nanos.to_le_bytes();
                let price_bytes = market_trade.price.to_le_bytes();
                let size_bytes = market_trade.size.to_le_bytes();
                let side_bytes = [market_trade.side, 0, 0, 0];

                let write_futures = vec![
                    handles.trade_id_file.write_all(&trade_id_bytes),
                    handles.trade_time_file.write_all(&time_bytes),
                    handles.trade_nanos_file.write_all(&nanos_bytes),
                    handles.trade_price_file.write_all(&price_bytes),
                    handles.trade_size_file.write_all(&size_bytes),
                    handles.trade_side_file.write_all(&side_bytes),
                    handles.maker_order_id_file.write_all(&market_trade.maker_order_id),
                    handles.taker_order_id_file.write_all(&market_trade.taker_order_id),
                ];

                try_join_all(write_futures).await?;
            }
        }

        // Flush trade files
        let trade_flush_futures: Vec<_> = self
            .trade_file_handles
            .values_mut()
            .map(|handles| handles.flush_all())
            .collect();

        try_join_all(trade_flush_futures).await?;

        // Flush market trade files
        let market_trade_flush_futures: Vec<_> = self
            .market_trade_file_handles
            .values_mut()
            .map(|handles| handles.flush_all())
            .collect();

        try_join_all(market_trade_flush_futures).await?;

        self.buffer.clear();
        self.trade_buffer.clear();
        self.market_trades_buffer.clear();
        Ok(())
    }

    pub async fn cleanup(&mut self) -> Result<()> {
        if let Err(e) = self.flush_buffer().await {
            eprintln!(
                "Connection {}: Error flushing buffer during cleanup: {}",
                self.connection_id, e
            );
        }

        // Close ticker file handles
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

        // Close trade file handles
        let trade_symbols: Vec<String> = self.trade_file_handles.keys().cloned().collect();
        for symbol in trade_symbols {
            if let Some(handles) = self.trade_file_handles.remove(&symbol) {
                if let Err(e) = handles.close().await {
                    eprintln!(
                        "Connection {}: Error closing trade file handles for {}: {}",
                        self.connection_id, symbol, e
                    );
                }
            }
        }

        // Close market trade file handles
        let market_trade_symbols: Vec<String> = self.market_trade_file_handles.keys().cloned().collect();
        for symbol in market_trade_symbols {
            if let Some(handles) = self.market_trade_file_handles.remove(&symbol) {
                if let Err(e) = handles.close().await {
                    eprintln!(
                        "Connection {}: Error closing market trade file handles for {}: {}",
                        self.connection_id, symbol, e
                    );
                }
            }
        }

        self.file_handles.clear();
        self.trade_file_handles.clear();
        self.market_trade_file_handles.clear();

        Ok(())
    }

    pub async fn recreate_file_handles(&mut self) -> Result<()> {
        let date = Local::now().format("%d.%m.%y").to_string();
        let mut new_file_handles = HashMap::new();
        let mut new_trade_file_handles = HashMap::new();
        let mut new_market_trade_file_handles = HashMap::new();

        for symbol in &self.symbols {
            let base_path = format!("/mnt/md/data/{symbol}");

            // Recreate ticker file handles
            match Self::create_file_handles_for_symbol(&base_path, &date).await {
                Ok(handles) => {
                    new_file_handles.insert(symbol.clone(), handles);
                }
                Err(e) => {
                    eprintln!(
                        "Connection {}: Failed to recreate file handles for {}: {}",
                        self.connection_id, symbol, e
                    );

                    // Cleanup on error
                    for (sym, handles) in new_file_handles {
                        if let Err(close_err) = handles.close().await {
                            eprintln!("Connection {}: Error closing file handles for {} during cleanup: {}", self.connection_id, sym, close_err);
                        }
                    }

                    return Err(e);
                }
            }

            // Recreate trade file handles
            match Self::create_trade_file_handles_for_symbol(&base_path, &date).await {
                Ok(handles) => {
                    new_trade_file_handles.insert(symbol.clone(), handles);
                }
                Err(e) => {
                    eprintln!(
                        "Connection {}: Failed to recreate trade file handles for {}: {}",
                        self.connection_id, symbol, e
                    );

                    // Cleanup all handles on error
                    for (sym, handles) in new_file_handles {
                        if let Err(close_err) = handles.close().await {
                            eprintln!("Connection {}: Error closing file handles for {} during cleanup: {}", self.connection_id, sym, close_err);
                        }
                    }
                    for (sym, handles) in new_trade_file_handles {
                        if let Err(close_err) = handles.close().await {
                            eprintln!("Connection {}: Error closing trade file handles for {} during cleanup: {}", self.connection_id, sym, close_err);
                        }
                    }

                    return Err(e);
                }
            }

            // Recreate market trade file handles
            match Self::create_market_trade_file_handles_for_symbol(&base_path, &date).await {
                Ok(handles) => {
                    new_market_trade_file_handles.insert(symbol.clone(), handles);
                }
                Err(e) => {
                    eprintln!(
                        "Connection {}: Failed to recreate market trade file handles for {}: {}",
                        self.connection_id, symbol, e
                    );

                    // Cleanup all handles on error
                    for (sym, handles) in new_file_handles {
                        if let Err(close_err) = handles.close().await {
                            eprintln!("Connection {}: Error closing file handles for {} during cleanup: {}", self.connection_id, sym, close_err);
                        }
                    }
                    for (sym, handles) in new_trade_file_handles {
                        if let Err(close_err) = handles.close().await {
                            eprintln!("Connection {}: Error closing trade file handles for {} during cleanup: {}", self.connection_id, sym, close_err);
                        }
                    }
                    for (sym, handles) in new_market_trade_file_handles {
                        if let Err(close_err) = handles.close().await {
                            eprintln!("Connection {}: Error closing market trade file handles for {} during cleanup: {}", self.connection_id, sym, close_err);
                        }
                    }

                    return Err(e);
                }
            }
        }

        self.file_handles = new_file_handles;
        self.trade_file_handles = new_trade_file_handles;
        self.market_trade_file_handles = new_market_trade_file_handles;
        self.current_date = date;
        Ok(())
    }

    pub fn update_reconnect_delay(&mut self) {
        self.reconnect_delay = std::cmp::min(self.reconnect_delay * 2, MAX_RECONNECT_DELAY);
    }

    pub fn reset_reconnect_delay(&mut self) {
        self.reconnect_delay = Duration::from_secs(1);
    }

    pub async fn check_and_rotate_files(&mut self) -> Result<()> {
        let current_date = Local::now().format("%d.%m.%y").to_string();
        
        // Check if date has changed (midnight has passed)
        if current_date != self.current_date {
            println!(
                "Connection {}: Date changed from {} to {}, rotating files...",
                self.connection_id, self.current_date, current_date
            );
            
            // First flush any remaining data
            if let Err(e) = self.flush_buffer().await {
                eprintln!(
                    "Connection {}: Error flushing buffer before rotation: {}",
                    self.connection_id, e
                );
            }
            
            // Close all existing file handles
            let symbols: Vec<String> = self.file_handles.keys().cloned().collect();
            for symbol in symbols {
                if let Some(handles) = self.file_handles.remove(&symbol) {
                    if let Err(e) = handles.close().await {
                        eprintln!(
                            "Connection {}: Error closing file handles for {} during rotation: {}",
                            self.connection_id, symbol, e
                        );
                    }
                }
            }
            
            let trade_symbols: Vec<String> = self.trade_file_handles.keys().cloned().collect();
            for symbol in trade_symbols {
                if let Some(handles) = self.trade_file_handles.remove(&symbol) {
                    if let Err(e) = handles.close().await {
                        eprintln!(
                            "Connection {}: Error closing trade file handles for {} during rotation: {}",
                            self.connection_id, symbol, e
                        );
                    }
                }
            }
            
            let market_trade_symbols: Vec<String> = self.market_trade_file_handles.keys().cloned().collect();
            for symbol in market_trade_symbols {
                if let Some(handles) = self.market_trade_file_handles.remove(&symbol) {
                    if let Err(e) = handles.close().await {
                        eprintln!(
                            "Connection {}: Error closing market trade file handles for {} during rotation: {}",
                            self.connection_id, symbol, e
                        );
                    }
                }
            }
            
            // Clear the handle maps
            self.file_handles.clear();
            self.trade_file_handles.clear();
            self.market_trade_file_handles.clear();
            
            // Create new file handles with the new date
            match self.recreate_file_handles().await {
                Ok(()) => {
                    println!(
                        "Connection {}: Successfully rotated files for new date {}",
                        self.connection_id, current_date
                    );
                }
                Err(e) => {
                    eprintln!(
                        "Connection {}: Failed to create new file handles after rotation: {}",
                        self.connection_id, e
                    );
                    return Err(e);
                }
            }
        }
        
        Ok(())
    }
}
