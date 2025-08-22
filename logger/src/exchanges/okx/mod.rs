mod connection;
mod parser;

pub use connection::OkxConnection;

use crate::common::{
    data_types::{ExchangeId, Symbol, UnifiedMarketData, UnifiedTradeData},
    AnalyticsEngine, DataBuffer, MarketMetrics, MetricsBridge,
};
use crate::config::Config;
use crate::exchanges::{distribute_symbols, Channel, Exchange, ExchangeConnection, Message};
use anyhow::Result;
use async_trait::async_trait;
use parser::{parse_okx_ticker, parse_okx_trade};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, error, warn};

pub struct OkxExchange {
    config: Arc<Config>,
    data_buffer: Arc<DataBuffer>,
    analytics: Arc<AnalyticsEngine>,
    metrics: Arc<MetricsBridge>,
}

impl OkxExchange {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let data_buffer = Arc::new(DataBuffer::new(config.logger.data_path.clone()));
        let analytics = Arc::new(AnalyticsEngine::new(10000.0, Duration::from_secs(30)));
        let market_metrics = Arc::new(MarketMetrics::new());
        let metrics = Arc::new(MetricsBridge::new(market_metrics));
        
        // Set initial connection status
        metrics.record_connection_status("okx", false);

        Ok(Self {
            config,
            data_buffer,
            analytics,
            metrics,
        })
    }
}

#[async_trait]
impl Exchange for OkxExchange {
    fn name(&self) -> &'static str {
        "OKX"
    }

    fn id(&self) -> ExchangeId {
        ExchangeId::OKX
    }

    async fn fetch_symbols(&self) -> Result<Vec<Symbol>> {
        let client = reqwest::Client::new();
        let url = format!(
            "{}/public/instruments?instType=SPOT",
            self.config.exchanges.okx.rest_endpoint
        );

        let response = client
            .get(&url)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let mut symbols = Vec::new();

        // OKX API response format: {"code":"0","msg":"","data":[...]}
        if let (Some(code), Some(data)) = (response["code"].as_str(), response["data"].as_array()) {
            if code != "0" {
                return Err(anyhow::anyhow!(
                    "OKX API error: {}",
                    response["msg"].as_str().unwrap_or("Unknown")
                ));
            }

            for instrument in data {
                if let (Some(inst_id), Some(base_ccy), Some(quote_ccy), Some(state)) = (
                    instrument["instId"].as_str(),
                    instrument["baseCcy"].as_str(),
                    instrument["quoteCcy"].as_str(),
                    instrument["state"].as_str(),
                ) {
                    if state == "live" {
                        let symbol = Symbol {
                            exchange: ExchangeId::OKX,
                            symbol: inst_id.to_string(),
                            base_asset: base_ccy.to_string(),
                            quote_asset: quote_ccy.to_string(),
                            asset_class: crate::common::data_types::AssetClass::Spot,
                            active: true,
                            min_size: instrument["minSz"].as_str().and_then(|s| s.parse().ok()),
                            tick_size: instrument["tickSz"].as_str().and_then(|s| s.parse().ok()),
                        };

                        symbols.push(symbol);
                    }
                }
            }
        }

        debug!("Fetched {} active symbols from OKX", symbols.len());
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
        Ok(Box::new(OkxConnection::new(
            self.config.exchanges.okx.ws_endpoint.clone(),
            symbols,
            data_sender,
        )))
    }

    fn parse_market_data(&self, raw: &Value) -> Result<Option<UnifiedMarketData>> {
        parse_okx_ticker(raw)
    }

    fn parse_trade_data(&self, raw: &Value) -> Result<Option<UnifiedTradeData>> {
        parse_okx_trade(raw)
    }

    fn max_symbols_per_connection(&self) -> usize {
        self.config.exchanges.okx.symbols_per_connection
    }

    fn max_connections(&self) -> usize {
        self.config.exchanges.okx.max_connections
    }

    async fn run(&self) -> Result<()> {
        debug!("Starting OKX exchange logger");

        // Fetch all symbols
        let symbols = if let Some(ref configured_symbols) = self.config.exchanges.okx.symbols {
            configured_symbols.clone()
        } else {
            self.fetch_symbols()
                .await?
                .into_iter()
                .map(|s| s.symbol)
                .collect()
        };

        debug!("Will monitor {} OKX symbols", symbols.len());

        // Distribute symbols across connections
        let symbol_batches = distribute_symbols(symbols, self.max_symbols_per_connection()).await;

        let (data_tx, mut data_rx) = mpsc::channel(10000);

        // Spawn connection handlers
        let mut connection_handles = Vec::new();
        for (idx, batch) in symbol_batches.into_iter().enumerate() {
            let data_tx = data_tx.clone();
            let config = self.config.clone();
            let metrics = self.metrics.clone();

            let handle = tokio::spawn(async move {
                let batch_owned = batch;
                let data_tx_owned = data_tx;
                
                let mut connection =
                    OkxConnection::new(config.exchanges.okx.ws_endpoint.clone(), batch_owned.clone(), data_tx_owned.clone());
                
                let mut consecutive_failures = 0;
                let max_consecutive_failures = 10;
                let mut backoff_secs = 1u64;
                let max_backoff_secs = 60u64;

                loop {
                    // Attempting to connect (actual connection logging happens in connect())

                    if let Err(e) = connection.connect().await {
                        error!("Connection {} failed to connect: {}", idx, e);
                        metrics.record_error("okx", e.to_string());
                        metrics.record_connection_status("okx", false);
                        
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

                    if let Err(e) = connection
                        .subscribe(vec![Channel::Ticker, Channel::Trades])
                        .await
                    {
                        error!("Connection {} failed to subscribe: {}", idx, e);
                        consecutive_failures += 1;
                        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                        backoff_secs = (backoff_secs * 2).min(max_backoff_secs);
                        continue;
                    }

                    // Successfully connected and subscribed
                    debug!("Connection {} successfully connected and subscribed to OKX", idx);
                    metrics.record_connection_status("okx", true);
                    consecutive_failures = 0;
                    backoff_secs = 1;

                    // Spawn ping task for OKX (required every 30 seconds)
                    let mut ping_connection = connection.clone_for_ping();
                    let ping_handle = tokio::spawn(async move {
                        let mut interval = interval(Duration::from_secs(20)); // Send ping every 20s to be safe
                        let mut ping_failures = 0;
                        loop {
                            interval.tick().await;
                            if let Err(e) = ping_connection.send_ping().await {
                                error!("Failed to send ping: {}", e);
                                ping_failures += 1;
                                if ping_failures >= 3 {
                                    error!("Too many ping failures, disconnecting");
                                    break;
                                }
                            } else {
                                ping_failures = 0;
                            }
                        }
                    });

                    // Read messages with timeout detection
                    let mut last_message_time = tokio::time::Instant::now();
                    let timeout_duration = Duration::from_secs(120); // 2 minute timeout
                    
                    loop {
                        // Check for timeout
                        if last_message_time.elapsed() > timeout_duration {
                            warn!("Connection {} timed out - no messages received for 2 minutes", idx);
                            metrics.record_error("okx", "Connection timeout".to_string());
                            break;
                        }

                        // Use timeout for reading messages
                        match tokio::time::timeout(Duration::from_secs(30), connection.read_message()).await {
                            Ok(Ok(Some(_msg))) => {
                                metrics.record_message("okx");
                                last_message_time = tokio::time::Instant::now();
                            }
                            Ok(Ok(None)) => {
                                // Connection closed
                                warn!("Connection {} closed by server", idx);
                                break;
                            }
                            Ok(Err(e)) => {
                                error!("Connection {} read error: {}", idx, e);
                                metrics.record_error("okx", e.to_string());
                                
                                // Check if it's a connection reset error
                                if e.to_string().contains("Connection reset") || e.to_string().contains("os error 104") {
                                    warn!("Connection {} reset by peer, will reconnect immediately", idx);
                                    consecutive_failures += 1;
                                }
                                break;
                            }
                            Err(_) => {
                                // Read timeout - continue to check overall timeout
                                continue;
                            }
                        }
                    }

                    // Cancel ping task
                    ping_handle.abort();
                    let _ = ping_handle.await; // Wait for task to finish

                    warn!("Connection {} disconnected, will reconnect", idx);
                    metrics.record_reconnect("okx");
                    metrics.record_connection_status("okx", false);
                    
                    // Clean disconnect before reconnecting
                    drop(connection);
                    connection = OkxConnection::new(
                        config.exchanges.okx.ws_endpoint.clone(),
                        batch_owned.clone(),
                        data_tx_owned.clone(),
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
                    error!("Failed to flush data: {}", e);
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
