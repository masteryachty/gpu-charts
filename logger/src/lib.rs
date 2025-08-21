pub mod common;
pub mod config;
pub mod exchanges;
pub mod metrics_server;

pub use common::*;
pub use config::Config;
pub use exchanges::{Exchange, ExchangeConnection};

use anyhow::Result;
use std::sync::Arc;

pub struct Logger {
    _config: Arc<Config>,
    exchanges: Vec<Arc<dyn Exchange>>,
}

impl Logger {
    pub fn new(config: Config) -> Result<Self> {
        let config = Arc::new(config);
        let mut exchanges: Vec<Arc<dyn Exchange>> = Vec::new();

        // Initialize exchanges based on config
        if config.exchanges.coinbase.enabled {
            exchanges.push(Arc::new(exchanges::coinbase::CoinbaseExchange::new(
                config.clone(),
            )?));
        }

        if config.exchanges.binance.enabled {
            exchanges.push(Arc::new(exchanges::binance::BinanceExchange::new(
                config.clone(),
            )?));
        }

        if config.exchanges.okx.enabled {
            exchanges.push(Arc::new(exchanges::okx::OkxExchange::new(config.clone())?));
        }

        if config.exchanges.kraken.enabled {
            exchanges.push(Arc::new(exchanges::kraken::KrakenExchange::new(
                config.clone(),
            )?));
        }

        if config.exchanges.bitfinex.enabled {
            exchanges.push(Arc::new(exchanges::bitfinex::BitfinexExchange::new(
                config.clone(),
            )?));
        }

        Ok(Self {
            _config: config,
            exchanges,
        })
    }

    pub async fn run(&self) -> Result<()> {
        let mut handles = Vec::new();

        for exchange in &self.exchanges {
            let exchange = exchange.clone();
            let handle = tokio::spawn(async move {
                if let Err(e) = exchange.run().await {
                    tracing::error!("Exchange {} failed: {}", exchange.name(), e);
                }
            });
            handles.push(handle);
        }

        // Wait for all exchanges
        for handle in handles {
            handle.await?;
        }

        Ok(())
    }
}
