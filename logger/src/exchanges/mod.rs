pub mod binance;
pub mod coinbase;

use crate::common::data_types::{ExchangeId, Symbol, UnifiedMarketData, UnifiedTradeData};
use crate::config::Config;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum Channel {
    Ticker,
    Trades,
    OrderBook,
}

#[derive(Debug)]
pub enum Message {
    MarketData(UnifiedMarketData),
    Trade(UnifiedTradeData),
    Heartbeat,
    Error(String),
}

#[async_trait]
pub trait Exchange: Send + Sync {
    // Exchange identification
    fn name(&self) -> &'static str;
    fn id(&self) -> ExchangeId;

    // Symbol management
    async fn fetch_symbols(&self) -> Result<Vec<Symbol>>;
    fn normalize_symbol(&self, exchange_symbol: &str) -> String;
    fn denormalize_symbol(&self, normalized_symbol: &str) -> String;

    // WebSocket management
    async fn create_connection(
        &self,
        symbols: Vec<String>,
        data_sender: mpsc::Sender<Message>,
    ) -> Result<Box<dyn ExchangeConnection>>;

    // Data parsing
    fn parse_market_data(&self, raw: &Value) -> Result<Option<UnifiedMarketData>>;
    fn parse_trade_data(&self, raw: &Value) -> Result<Option<UnifiedTradeData>>;

    // Configuration
    fn max_symbols_per_connection(&self) -> usize;
    fn max_connections(&self) -> usize;

    // Run the exchange
    async fn run(&self) -> Result<()>;
}

#[async_trait]
pub trait ExchangeConnection: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn subscribe(&mut self, channels: Vec<Channel>) -> Result<()>;
    async fn read_message(&mut self) -> Result<Option<Value>>;
    async fn send_ping(&mut self) -> Result<()>;
    async fn reconnect(&mut self) -> Result<()>;
    fn is_connected(&self) -> bool;
    fn symbols(&self) -> &[String];
}

pub async fn distribute_symbols(
    total_symbols: Vec<String>,
    max_per_connection: usize,
) -> Vec<Vec<String>> {
    let mut distributions = Vec::new();
    let mut current_batch = Vec::new();

    for symbol in total_symbols {
        current_batch.push(symbol);
        if current_batch.len() >= max_per_connection {
            distributions.push(current_batch);
            current_batch = Vec::new();
        }
    }

    if !current_batch.is_empty() {
        distributions.push(current_batch);
    }

    distributions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_distribute_symbols() {
        let symbols: Vec<String> = (0..25).map(|i| format!("SYM{}", i)).collect();
        let distributions = distribute_symbols(symbols, 10).await;

        assert_eq!(distributions.len(), 3);
        assert_eq!(distributions[0].len(), 10);
        assert_eq!(distributions[1].len(), 10);
        assert_eq!(distributions[2].len(), 5);
    }
}
