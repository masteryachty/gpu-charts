mod connection;
mod parser;

pub use connection::BitfinexConnection;

use crate::common::{
    data_types::{ExchangeId, Symbol, UnifiedMarketData, UnifiedTradeData},
    AnalyticsEngine, DataBuffer, MarketMetrics,
};
use crate::config::Config;
use crate::exchanges::{distribute_symbols, Channel, Exchange, ExchangeConnection, Message};
use anyhow::Result;
use async_trait::async_trait;
use parser::{parse_bitfinex_ticker, parse_bitfinex_trade};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{error, info};

pub struct BitfinexExchange {
    config: Arc<Config>,
    data_buffer: Arc<DataBuffer>,
    analytics: Arc<AnalyticsEngine>,
    metrics: Arc<MarketMetrics>,
}

impl BitfinexExchange {
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
impl Exchange for BitfinexExchange {
    fn name(&self) -> &'static str {
        "Bitfinex"
    }

    fn id(&self) -> ExchangeId {
        ExchangeId::Bitfinex
    }

    async fn fetch_symbols(&self) -> Result<Vec<Symbol>> {
        let client = reqwest::Client::new();
        let url = format!(
            "{}/conf/pub:list:pair:exchange",
            self.config.exchanges.bitfinex.rest_endpoint
        );

        let response = client
            .get(&url)
            .send()
            .await?
            .json::<Vec<Vec<String>>>()
            .await?;

        let mut symbols = Vec::new();

        // Bitfinex returns array of arrays, first element is the list of symbols
        if let Some(symbol_list) = response.first() {
            for symbol_str in symbol_list {
                // Skip funding pairs (starting with 'f')
                if symbol_str.starts_with('f') {
                    continue;
                }

                let symbol = Symbol {
                    exchange: ExchangeId::Bitfinex,
                    symbol: format!("t{}", symbol_str.to_uppercase()),
                    base_asset: String::new(), // We'll store empty for now as we're not parsing
                    quote_asset: String::new(), // We'll store empty for now as we're not parsing
                    asset_class: crate::common::data_types::AssetClass::Spot,
                    active: true,
                    min_size: None,
                    tick_size: None,
                };

                symbols.push(symbol);
            }
        }

        info!("Fetched {} active symbols from Bitfinex", symbols.len());
        Ok(symbols)
    }

    fn normalize_symbol(&self, exchange_symbol: &str) -> String {
        // Return as-is, no normalization
        exchange_symbol.to_string()
    }

    fn denormalize_symbol(&self, normalized_symbol: &str) -> String {
        // Return as-is, no denormalization
        normalized_symbol.to_string()
    }

    async fn create_connection(
        &self,
        symbols: Vec<String>,
        data_sender: mpsc::Sender<Message>,
    ) -> Result<Box<dyn ExchangeConnection>> {
        Ok(Box::new(BitfinexConnection::new(
            self.config.exchanges.bitfinex.ws_endpoint.clone(),
            symbols,
            data_sender,
            self.config.exchanges.bitfinex.ping_interval_secs,
        )))
    }

    fn parse_market_data(&self, raw: &Value) -> Result<Option<UnifiedMarketData>> {
        parse_bitfinex_ticker(raw)
    }

    fn parse_trade_data(&self, raw: &Value) -> Result<Option<UnifiedTradeData>> {
        parse_bitfinex_trade(raw)
    }

    fn max_symbols_per_connection(&self) -> usize {
        self.config.exchanges.bitfinex.symbols_per_connection
    }

    fn max_connections(&self) -> usize {
        self.config.exchanges.bitfinex.max_connections
    }

    async fn run(&self) -> Result<()> {
        info!("Starting Bitfinex exchange logger");

        // Fetch all symbols
        let symbols = if let Some(ref configured_symbols) = self.config.exchanges.bitfinex.symbols {
            configured_symbols.clone()
        } else {
            self.fetch_symbols()
                .await?
                .into_iter()
                .map(|s| s.symbol)
                .collect()
        };

        info!("Will monitor {} Bitfinex symbols", symbols.len());

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
                let mut connection = BitfinexConnection::new(
                    config.exchanges.bitfinex.ws_endpoint.clone(),
                    batch,
                    data_tx,
                    config.exchanges.bitfinex.ping_interval_secs,
                );

                loop {
                    metrics.record_connection_status("bitfinex", true);

                    if let Err(e) = connection.connect().await {
                        error!("Connection {} failed to connect: {}", idx, e);
                        metrics.record_error("bitfinex", e.to_string());
                        metrics.record_connection_status("bitfinex", false);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }

                    if let Err(e) = connection
                        .subscribe(vec![Channel::Ticker, Channel::Trades])
                        .await
                    {
                        error!("Connection {} failed to subscribe: {}", idx, e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }

                    // Create ping interval if configured
                    let mut ping_interval = config
                        .exchanges
                        .bitfinex
                        .ping_interval_secs
                        .map(|secs| interval(Duration::from_secs(secs)));

                    // Read messages with ping handling
                    loop {
                        if let Some(ref mut ping_int) = ping_interval {
                            tokio::select! {
                                _ = ping_int.tick() => {
                                    if let Err(e) = connection.send_ping().await {
                                        error!("Failed to send ping: {}", e);
                                        break;
                                    }
                                }
                                result = connection.read_message() => {
                                    match result {
                                        Ok(Some(_msg)) => {
                                            metrics.record_message("bitfinex");
                                        }
                                        Ok(None) => {
                                            // Connection closed
                                            break;
                                        }
                                        Err(e) => {
                                            error!("Connection {} read error: {}", idx, e);
                                            metrics.record_error("bitfinex", e.to_string());
                                            break;
                                        }
                                    }
                                }
                            }
                        } else {
                            match connection.read_message().await {
                                Ok(Some(_msg)) => {
                                    metrics.record_message("bitfinex");
                                }
                                Ok(None) => {
                                    // Connection closed
                                    break;
                                }
                                Err(e) => {
                                    error!("Connection {} read error: {}", idx, e);
                                    metrics.record_error("bitfinex", e.to_string());
                                    break;
                                }
                            }
                        }
                    }

                    metrics.record_reconnect("bitfinex");
                    tokio::time::sleep(Duration::from_secs(5)).await;
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
