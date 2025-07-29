mod connection;
mod parser;

pub use connection::OkxConnection;

use crate::common::{
    data_types::{ExchangeId, Symbol, UnifiedMarketData, UnifiedTradeData},
    utils::{denormalize_symbol_okx, normalize_symbol_okx},
    AnalyticsEngine, DataBuffer, MarketMetrics,
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
use tracing::{error, info};

pub struct OkxExchange {
    config: Arc<Config>,
    symbol_mapper: Arc<crate::common::SymbolMapper>,
    data_buffer: Arc<DataBuffer>,
    analytics: Arc<AnalyticsEngine>,
    metrics: Arc<MarketMetrics>,
}

impl OkxExchange {
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
                            exchange_symbol: inst_id.to_string(),
                            normalized: normalize_symbol_okx(inst_id),
                            base_asset: base_ccy.to_string(),
                            quote_asset: quote_ccy.to_string(),
                            asset_class: crate::common::data_types::AssetClass::Spot,
                            active: true,
                            min_size: instrument["minSz"].as_str().and_then(|s| s.parse().ok()),
                            tick_size: instrument["tickSz"].as_str().and_then(|s| s.parse().ok()),
                        };

                        // Add to symbol mapper
                        self.symbol_mapper.add_symbol(symbol.clone());
                        symbols.push(symbol);
                    }
                }
            }
        }

        info!("Fetched {} active symbols from OKX", symbols.len());
        Ok(symbols)
    }

    fn normalize_symbol(&self, exchange_symbol: &str) -> String {
        self.symbol_mapper
            .normalize(ExchangeId::OKX, exchange_symbol)
            .unwrap_or_else(|| normalize_symbol_okx(exchange_symbol))
    }

    fn denormalize_symbol(&self, normalized_symbol: &str) -> String {
        self.symbol_mapper
            .to_exchange(normalized_symbol, ExchangeId::OKX)
            .unwrap_or_else(|| denormalize_symbol_okx(normalized_symbol))
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
            self.symbol_mapper.clone(),
        )))
    }

    fn parse_market_data(&self, raw: &Value) -> Result<Option<UnifiedMarketData>> {
        parse_okx_ticker(raw, &self.symbol_mapper)
    }

    fn parse_trade_data(&self, raw: &Value) -> Result<Option<UnifiedTradeData>> {
        parse_okx_trade(raw, &self.symbol_mapper)
    }

    fn max_symbols_per_connection(&self) -> usize {
        self.config.exchanges.okx.symbols_per_connection
    }

    fn max_connections(&self) -> usize {
        self.config.exchanges.okx.max_connections
    }

    async fn run(&self) -> Result<()> {
        info!("Starting OKX exchange logger");

        // Fetch all symbols
        let symbols = if let Some(ref configured_symbols) = self.config.exchanges.okx.symbols {
            configured_symbols.clone()
        } else {
            self.fetch_symbols()
                .await?
                .into_iter()
                .map(|s| s.exchange_symbol)
                .collect()
        };

        info!("Will monitor {} OKX symbols", symbols.len());

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
                let mut connection = OkxConnection::new(
                    config.exchanges.okx.ws_endpoint.clone(),
                    batch,
                    data_tx,
                    symbol_mapper,
                );

                loop {
                    metrics.record_connection_status("okx", true);

                    if let Err(e) = connection.connect().await {
                        error!("Connection {} failed to connect: {}", idx, e);
                        metrics.record_error("okx", e.to_string());
                        metrics.record_connection_status("okx", false);
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

                    // Spawn ping task for OKX (required every 30 seconds)
                    let mut ping_connection = connection.clone_for_ping();
                    let ping_task = tokio::spawn(async move {
                        let mut interval = interval(Duration::from_secs(25)); // Send ping every 25s to be safe
                        loop {
                            interval.tick().await;
                            if let Err(e) = ping_connection.send_ping().await {
                                error!("Failed to send ping: {}", e);
                                break;
                            }
                        }
                    });

                    // Read messages
                    loop {
                        match connection.read_message().await {
                            Ok(Some(_msg)) => {
                                metrics.record_message("okx");
                            }
                            Ok(None) => {
                                // Connection closed
                                break;
                            }
                            Err(e) => {
                                error!("Connection {} read error: {}", idx, e);
                                metrics.record_error("okx", e.to_string());
                                break;
                            }
                        }
                    }

                    // Cancel ping task
                    ping_task.abort();

                    metrics.record_reconnect("okx");
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
