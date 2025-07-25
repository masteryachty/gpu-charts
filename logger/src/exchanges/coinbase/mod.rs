mod connection;
mod parser;

pub use connection::CoinbaseConnection;

use crate::common::{
    data_types::{ExchangeId, Symbol, UnifiedMarketData, UnifiedTradeData},
    utils::normalize_symbol_coinbase,
    AnalyticsEngine, DataBuffer, MarketMetrics,
};
use crate::config::Config;
use crate::exchanges::{distribute_symbols, Channel, Exchange, ExchangeConnection, Message};
use anyhow::Result;
use async_trait::async_trait;
use parser::{parse_coinbase_ticker, parse_coinbase_trade};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{error, info};

pub struct CoinbaseExchange {
    config: Arc<Config>,
    symbol_mapper: Arc<crate::common::SymbolMapper>,
    data_buffer: Arc<DataBuffer>,
    analytics: Arc<AnalyticsEngine>,
    metrics: Arc<MarketMetrics>,
}

impl CoinbaseExchange {
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
impl Exchange for CoinbaseExchange {
    fn name(&self) -> &'static str {
        "Coinbase"
    }

    fn id(&self) -> ExchangeId {
        ExchangeId::Coinbase
    }

    async fn fetch_symbols(&self) -> Result<Vec<Symbol>> {
        let client = reqwest::Client::new();
        let url = format!("{}/products", self.config.exchanges.coinbase.rest_endpoint);

        let response = client
            .get(&url)
            .send()
            .await?
            .json::<Vec<serde_json::Value>>()
            .await?;

        let mut symbols = Vec::new();

        for product in response {
            if let (Some(id), Some(base), Some(quote), Some(status)) = (
                product["id"].as_str(),
                product["base_currency"].as_str(),
                product["quote_currency"].as_str(),
                product["status"].as_str(),
            ) {
                if status == "online" {
                    let symbol = Symbol {
                        exchange: ExchangeId::Coinbase,
                        exchange_symbol: id.to_string(),
                        normalized: normalize_symbol_coinbase(id),
                        base_asset: base.to_string(),
                        quote_asset: quote.to_string(),
                        asset_class: crate::common::data_types::AssetClass::Spot,
                        active: true,
                        min_size: product["base_min_size"]
                            .as_str()
                            .and_then(|s| s.parse().ok()),
                        tick_size: product["quote_increment"]
                            .as_str()
                            .and_then(|s| s.parse().ok()),
                    };

                    // Add to symbol mapper
                    self.symbol_mapper.add_symbol(symbol.clone());
                    symbols.push(symbol);
                }
            }
        }

        info!("Fetched {} active symbols from Coinbase", symbols.len());
        Ok(symbols)
    }

    fn normalize_symbol(&self, exchange_symbol: &str) -> String {
        self.symbol_mapper
            .normalize(ExchangeId::Coinbase, exchange_symbol)
            .unwrap_or_else(|| normalize_symbol_coinbase(exchange_symbol))
    }

    fn denormalize_symbol(&self, normalized_symbol: &str) -> String {
        self.symbol_mapper
            .to_exchange(normalized_symbol, ExchangeId::Coinbase)
            .unwrap_or_else(|| normalized_symbol.to_string())
    }

    async fn create_connection(
        &self,
        symbols: Vec<String>,
        data_sender: mpsc::Sender<Message>,
    ) -> Result<Box<dyn ExchangeConnection>> {
        Ok(Box::new(CoinbaseConnection::new(
            self.config.exchanges.coinbase.ws_endpoint.clone(),
            symbols,
            data_sender,
            self.symbol_mapper.clone(),
        )))
    }

    fn parse_market_data(&self, raw: &Value) -> Result<Option<UnifiedMarketData>> {
        parse_coinbase_ticker(raw, &self.symbol_mapper)
    }

    fn parse_trade_data(&self, raw: &Value) -> Result<Option<UnifiedTradeData>> {
        parse_coinbase_trade(raw, &self.symbol_mapper)
    }

    fn max_symbols_per_connection(&self) -> usize {
        self.config.exchanges.coinbase.symbols_per_connection
    }

    fn max_connections(&self) -> usize {
        self.config.exchanges.coinbase.max_connections
    }

    async fn run(&self) -> Result<()> {
        info!("Starting Coinbase exchange logger");

        // Fetch all symbols
        let symbols = if let Some(ref configured_symbols) = self.config.exchanges.coinbase.symbols {
            configured_symbols.clone()
        } else {
            self.fetch_symbols()
                .await?
                .into_iter()
                .map(|s| s.exchange_symbol)
                .collect()
        };

        info!("Will monitor {} Coinbase symbols", symbols.len());

        // Distribute symbols across connections
        let symbol_batches = distribute_symbols(symbols, self.max_symbols_per_connection()).await;

        let (data_tx, mut data_rx) = mpsc::channel(10000);

        // Spawn connection handlers
        let mut connection_handles = Vec::new();
        for (idx, batch) in symbol_batches.into_iter().enumerate() {
            let data_tx = data_tx.clone();
            let config = self.config.clone();
            let metrics = self.metrics.clone();
            let symbol_mapper = self.symbol_mapper.clone();

            let handle = tokio::spawn(async move {
                let mut connection = CoinbaseConnection::new(
                    config.exchanges.coinbase.ws_endpoint.clone(),
                    batch,
                    data_tx,
                    symbol_mapper,
                );

                loop {
                    metrics.record_connection_status("coinbase", true);

                    if let Err(e) = connection.connect().await {
                        error!("Connection {} failed to connect: {}", idx, e);
                        metrics.record_error("coinbase", e.to_string());
                        metrics.record_connection_status("coinbase", false);
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

                    // Read messages
                    loop {
                        match connection.read_message().await {
                            Ok(Some(_msg)) => {
                                metrics.record_message("coinbase");
                            }
                            Ok(None) => {
                                // Connection closed
                                break;
                            }
                            Err(e) => {
                                error!("Connection {} read error: {}", idx, e);
                                metrics.record_error("coinbase", e.to_string());
                                break;
                            }
                        }
                    }

                    metrics.record_reconnect("coinbase");
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
