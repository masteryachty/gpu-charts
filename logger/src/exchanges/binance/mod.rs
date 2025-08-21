mod connection;
mod parser;

pub use connection::BinanceConnection;

use crate::common::{
    data_types::{ExchangeId, Symbol, UnifiedMarketData, UnifiedTradeData},
    AnalyticsEngine, DataBuffer, MarketMetrics,
};
use crate::config::Config;
use crate::exchanges::{distribute_symbols, Channel, Exchange, ExchangeConnection, Message};
use anyhow::Result;
use async_trait::async_trait;
use parser::{parse_binance_ticker, parse_binance_trade};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

pub struct BinanceExchange {
    config: Arc<Config>,
    data_buffer: Arc<DataBuffer>,
    analytics: Arc<AnalyticsEngine>,
    metrics: Arc<MarketMetrics>,
}

impl BinanceExchange {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let data_buffer = Arc::new(DataBuffer::new(config.logger.data_path.clone()));
        let analytics = Arc::new(AnalyticsEngine::new(10000.0, Duration::from_secs(30)));
        let metrics = Arc::new(MarketMetrics::new());

        Ok(Self {
            config,
            data_buffer,
            analytics,
            metrics,
        })
    }
}

#[async_trait]
impl Exchange for BinanceExchange {
    fn name(&self) -> &'static str {
        "Binance"
    }

    fn id(&self) -> ExchangeId {
        ExchangeId::Binance
    }

    async fn fetch_symbols(&self) -> Result<Vec<Symbol>> {
        let client = reqwest::Client::new();
        let url = format!(
            "{}/api/v3/exchangeInfo",
            self.config.exchanges.binance.rest_endpoint
        );

        let response = client
            .get(&url)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let mut symbols = Vec::new();

        if let Some(symbols_array) = response["symbols"].as_array() {
            for symbol_obj in symbols_array {
                if let (Some(symbol), Some(base), Some(quote), Some(status)) = (
                    symbol_obj["symbol"].as_str(),
                    symbol_obj["baseAsset"].as_str(),
                    symbol_obj["quoteAsset"].as_str(),
                    symbol_obj["status"].as_str(),
                ) {
                    if status == "TRADING" {
                        let symbol = Symbol {
                            exchange: ExchangeId::Binance,
                            symbol: symbol.to_string(),
                            base_asset: base.to_string(),
                            quote_asset: quote.to_string(),
                            asset_class: crate::common::data_types::AssetClass::Spot,
                            active: true,
                            min_size: None,  // Could parse from filters
                            tick_size: None, // Could parse from filters
                        };

                        symbols.push(symbol);
                    }
                }
            }
        }

        info!("Fetched {} active symbols from Binance", symbols.len());
        Ok(symbols)
    }

    fn normalize_symbol(&self, exchange_symbol: &str) -> String {
        exchange_symbol.to_string()
    }

    fn denormalize_symbol(&self, normalized_symbol: &str) -> String {
        normalized_symbol.to_string()
    }

    async fn create_connection(
        &self,
        symbols: Vec<String>,
        data_sender: mpsc::Sender<Message>,
    ) -> Result<Box<dyn ExchangeConnection>> {
        Ok(Box::new(BinanceConnection::new(
            self.config.exchanges.binance.ws_endpoint.clone(),
            symbols,
            data_sender,
            self.config
                .exchanges
                .binance
                .ping_interval_secs
                .unwrap_or(20),
        )))
    }

    fn parse_market_data(&self, raw: &Value) -> Result<Option<UnifiedMarketData>> {
        parse_binance_ticker(raw)
    }

    fn parse_trade_data(&self, raw: &Value) -> Result<Option<UnifiedTradeData>> {
        parse_binance_trade(raw)
    }

    fn max_symbols_per_connection(&self) -> usize {
        self.config.exchanges.binance.symbols_per_connection
    }

    fn max_connections(&self) -> usize {
        self.config.exchanges.binance.max_connections
    }

    async fn run(&self) -> Result<()> {
        info!("Starting Binance exchange logger");

        // Fetch all symbols
        let symbols = if let Some(ref configured_symbols) = self.config.exchanges.binance.symbols {
            configured_symbols.clone()
        } else {
            self.fetch_symbols()
                .await?
                .into_iter()
                .map(|s| s.symbol)
                .collect()
        };

        info!("Will monitor {} Binance symbols", symbols.len());

        // Distribute symbols across connections
        let symbol_batches = distribute_symbols(symbols, self.max_symbols_per_connection()).await;

        let (data_tx, mut data_rx) = mpsc::channel(10000);

        // Spawn connection handlers
        let mut connection_handles = Vec::new();
        for (idx, batch) in symbol_batches.into_iter().enumerate() {
            let data_tx = data_tx.clone();
            let config = self.config.clone();
            let metrics = self.metrics.clone();
            let ping_interval = self
                .config
                .exchanges
                .binance
                .ping_interval_secs
                .unwrap_or(20);

            let handle = tokio::spawn(async move {
                let batch_owned = batch;
                let data_tx_owned = data_tx;
                
                let mut connection = BinanceConnection::new(
                    config.exchanges.binance.ws_endpoint.clone(),
                    batch_owned.clone(),
                    data_tx_owned.clone(),
                    ping_interval,
                );
                
                let mut consecutive_failures = 0;
                let max_consecutive_failures = 10;
                let mut backoff_secs = 1u64;
                let max_backoff_secs = 60u64;

                loop {
                    info!("Connection {} attempting to connect to Binance", idx);
                    
                    // For Binance, connect and subscribe are combined
                    if let Err(e) = connection
                        .subscribe(vec![Channel::Ticker, Channel::Trades])
                        .await
                    {
                        error!("Connection {} failed to connect and subscribe: {}", idx, e);
                        metrics.record_error("binance", e.to_string());
                        metrics.record_connection_status("binance", false);
                        
                        consecutive_failures += 1;
                        if consecutive_failures >= max_consecutive_failures {
                            error!("Connection {} exceeded max consecutive failures ({}), waiting longer", idx, max_consecutive_failures);
                            tokio::time::sleep(Duration::from_secs(300)).await; // Wait 5 minutes
                            consecutive_failures = 0;
                            backoff_secs = 1;
                        } else {
                            tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                            backoff_secs = (backoff_secs * 2).min(max_backoff_secs);
                        }
                        continue;
                    }

                    info!("Connection {} successfully connected and subscribed to Binance", idx);
                    metrics.record_connection_status("binance", true);
                    consecutive_failures = 0;
                    backoff_secs = 1;

                    // Spawn ping task for Binance
                    let mut ping_interval = interval(Duration::from_secs(ping_interval));
                    let ping_task = {
                        let mut conn_clone = connection.clone();
                        tokio::spawn(async move {
                            loop {
                                ping_interval.tick().await;
                                if let Err(e) = conn_clone.send_ping().await {
                                    error!("Failed to send ping: {}", e);
                                    break;
                                }
                            }
                        })
                    };

                    // Read messages
                    info!("Connection {} started reading messages", idx);
                    let mut last_message_time = tokio::time::Instant::now();
                    let timeout_duration = Duration::from_secs(60); // 60 second timeout

                    loop {
                        // Use timeout to avoid indefinite blocking
                        let read_timeout = Duration::from_secs(30);
                        let read_future =
                            tokio::time::timeout(read_timeout, connection.read_message());

                        match read_future.await {
                            Ok(Ok(Some(_msg))) => {
                                metrics.record_message("binance");
                                last_message_time = tokio::time::Instant::now();
                            }
                            Ok(Ok(None)) => {
                                // Some message types return None (ping/pong, etc.)
                                // This is normal, just continue
                                continue;
                            }
                            Ok(Err(e)) => {
                                error!("Connection {} read error: {}", idx, e);
                                metrics.record_error("binance", e.to_string());
                                break;
                            }
                            Err(_) => {
                                // Timeout reading message
                                if last_message_time.elapsed() > timeout_duration {
                                    warn!(
                                        "Connection {} timed out - no messages for 60 seconds",
                                        idx
                                    );
                                    break;
                                }
                                // Otherwise, just continue - Binance might be slow
                                debug!(
                                    "Connection {} read timeout, but within acceptable window",
                                    idx
                                );
                            }
                        }
                    }

                    // Cancel ping task
                    ping_task.abort();
                    let _ = ping_task.await; // Wait for task to finish

                    warn!("Connection {} disconnected, will reconnect", idx);
                    metrics.record_reconnect("binance");
                    metrics.record_connection_status("binance", false);
                    
                    // Clean disconnect before reconnecting
                    drop(connection);
                    connection = BinanceConnection::new(
                        config.exchanges.binance.ws_endpoint.clone(),
                        batch_owned.clone(),
                        data_tx_owned.clone(),
                        config.exchanges.binance.ping_interval_secs.unwrap_or(20),
                    );
                    
                    tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(max_backoff_secs);
                }
            });

            connection_handles.push(handle);
        }

        // Spawn data processor
        let data_buffer = self.data_buffer.clone();
        let analytics = self.analytics.clone();
        let data_processor = tokio::spawn(async move {
            while let Some(message) = data_rx.recv().await {
                match message {
                    Message::MarketData(data) => {
                        if let Err(e) = data_buffer.add_market_data(data).await {
                            error!("Failed to buffer market data: {}", e);
                        }
                    }
                    Message::Trade(data) => {
                        analytics.process_trade(&data);
                        if let Err(e) = data_buffer.add_trade_data(data).await {
                            error!("Failed to buffer trade data: {}", e);
                        }
                    }
                    Message::Heartbeat => {}
                    Message::Error(e) => {
                        error!("Exchange error: {}", e);
                    }
                }
            }
        });

        // Spawn periodic tasks
        let flush_interval = Duration::from_secs(self.config.logger.flush_interval_secs);
        let data_buffer_flush = self.data_buffer.clone();
        let analytics_report = self.analytics.clone();

        let flush_task = tokio::spawn(async move {
            let mut interval = interval(flush_interval);
            loop {
                interval.tick().await;

                if let Err(e) = data_buffer_flush.flush_to_disk().await {
                    // Log the error but continue running
                    error!("Failed to flush data: {}", e);

                    // Check if it's an I/O error and provide more context
                    if e.to_string().contains("Input/output error") {
                        error!("I/O error detected - possible disk issues. Data may be buffered in memory.");
                    }
                }

                if let Err(e) = data_buffer_flush.rotate_files_if_needed().await {
                    error!("Failed to rotate files: {}", e);
                }
            }
        });

        let analytics_task = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                analytics_report.print_report();
            }
        });

        // Wait for all tasks
        tokio::select! {
            _ = async {
                for handle in connection_handles {
                    let _ = handle.await;
                }
            } => {}
            _ = data_processor => {}
            _ = flush_task => {}
            _ = analytics_task => {}
        }

        Ok(())
    }
}
