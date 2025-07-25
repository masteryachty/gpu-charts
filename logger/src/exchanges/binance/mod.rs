mod connection;
mod parser;

pub use connection::BinanceConnection;

use crate::common::{
    data_types::{ExchangeId, Symbol, UnifiedMarketData, UnifiedTradeData},
    utils::{denormalize_symbol_binance, normalize_symbol_binance},
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
use tracing::{error, info};

pub struct BinanceExchange {
    config: Arc<Config>,
    symbol_mapper: Arc<crate::common::SymbolMapper>,
    data_buffer: Arc<DataBuffer>,
    analytics: Arc<AnalyticsEngine>,
    metrics: Arc<MarketMetrics>,
}

impl BinanceExchange {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let symbol_mapper = Arc::new(crate::common::SymbolMapper::new(
            config.symbol_mappings.clone(),
        )?);

        let data_buffer = Arc::new(DataBuffer::new(config.logger.data_path.clone()));
        let analytics = Arc::new(AnalyticsEngine::new(10000.0, Duration::from_secs(30)));
        let metrics = Arc::new(MarketMetrics::new());

        Ok(Self {
            config,
            symbol_mapper,
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
                        let normalized = normalize_symbol_binance(symbol);

                        let symbol = Symbol {
                            exchange: ExchangeId::Binance,
                            exchange_symbol: symbol.to_string(),
                            normalized: normalized.clone(),
                            base_asset: base.to_string(),
                            quote_asset: quote.to_string(),
                            asset_class: crate::common::data_types::AssetClass::Spot,
                            active: true,
                            min_size: None,  // Could parse from filters
                            tick_size: None, // Could parse from filters
                        };

                        // Add to symbol mapper
                        self.symbol_mapper.add_symbol(symbol.clone());
                        symbols.push(symbol);
                    }
                }
            }
        }

        info!("Fetched {} active symbols from Binance", symbols.len());
        Ok(symbols)
    }

    fn normalize_symbol(&self, exchange_symbol: &str) -> String {
        self.symbol_mapper
            .normalize(ExchangeId::Binance, exchange_symbol)
            .unwrap_or_else(|| normalize_symbol_binance(exchange_symbol))
    }

    fn denormalize_symbol(&self, normalized_symbol: &str) -> String {
        self.symbol_mapper
            .to_exchange(normalized_symbol, ExchangeId::Binance)
            .unwrap_or_else(|| denormalize_symbol_binance(normalized_symbol))
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
            self.symbol_mapper.clone(),
        )))
    }

    fn parse_market_data(&self, raw: &Value) -> Result<Option<UnifiedMarketData>> {
        parse_binance_ticker(raw, &*self.symbol_mapper)
    }

    fn parse_trade_data(&self, raw: &Value) -> Result<Option<UnifiedTradeData>> {
        parse_binance_trade(raw, &*self.symbol_mapper)
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
                .map(|s| s.exchange_symbol)
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
            let symbol_mapper = self.symbol_mapper.clone();

            let handle = tokio::spawn(async move {
                let mut connection = BinanceConnection::new(
                    config.exchanges.binance.ws_endpoint.clone(),
                    batch,
                    data_tx,
                    ping_interval,
                    symbol_mapper,
                );

                loop {
                    metrics.record_connection_status("binance", true);

                    if let Err(e) = connection.connect().await {
                        error!("Connection {} failed to connect: {}", idx, e);
                        metrics.record_error("binance", e.to_string());
                        metrics.record_connection_status("binance", false);
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
                    loop {
                        match connection.read_message().await {
                            Ok(Some(_msg)) => {
                                metrics.record_message("binance");
                            }
                            Ok(None) => {
                                // Connection closed
                                break;
                            }
                            Err(e) => {
                                error!("Connection {} read error: {}", idx, e);
                                metrics.record_error("binance", e.to_string());
                                break;
                            }
                        }
                    }

                    // Cancel ping task
                    ping_task.abort();

                    metrics.record_reconnect("binance");
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
