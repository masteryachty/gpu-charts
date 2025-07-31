use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExchangeId {
    Coinbase,
    Binance,
    Kraken,
    Bybit,
    OKX,
    Bitfinex,
}

impl ExchangeId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExchangeId::Coinbase => "coinbase",
            ExchangeId::Binance => "binance",
            ExchangeId::Kraken => "kraken",
            ExchangeId::Bybit => "bybit",
            ExchangeId::OKX => "okx",
            ExchangeId::Bitfinex => "bitfinex",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TradeSide {
    Buy = 0,
    Sell = 1,
}

impl TradeSide {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "buy" | "b" => Some(TradeSide::Buy),
            "sell" | "s" => Some(TradeSide::Sell),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> u32 {
        *self as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AssetClass {
    Spot,
    Futures,
    Options,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QuoteType {
    Fiat(String),
    Stablecoin(String),
    Crypto(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMarketData {
    pub exchange: ExchangeId,
    pub symbol: String,  // Raw exchange symbol (e.g., BTC-USD, BTCUSDT, etc.)
    pub timestamp: u32,  // Unix timestamp
    pub nanos: u32,      // Nanosecond precision
    pub price: f32,      // Last trade price
    pub volume: f32,     // Last trade volume
    pub side: TradeSide, // Buy/Sell
    pub best_bid: f32,   // Best bid price
    pub best_ask: f32,   // Best ask price
    pub exchange_specific: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTradeData {
    pub exchange: ExchangeId,
    pub symbol: String,           // Raw exchange symbol
    pub trade_id: u64,            // Exchange trade ID
    pub timestamp: u32,           // Unix timestamp
    pub nanos: u32,               // Nanosecond precision
    pub price: f32,               // Trade price
    pub size: f32,                // Trade size
    pub side: TradeSide,          // Buy/Sell
    pub maker_order_id: [u8; 16], // UUID bytes
    pub taker_order_id: [u8; 16], // UUID bytes
    pub exchange_specific: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub exchange: ExchangeId,
    pub symbol: String,          // Exchange-specific symbol
    pub base_asset: String,
    pub quote_asset: String,
    pub asset_class: AssetClass,
    pub active: bool,
    pub min_size: Option<f64>,
    pub tick_size: Option<f64>,
}

impl UnifiedMarketData {
    pub fn new(exchange: ExchangeId, symbol: String) -> Self {
        let now = Utc::now();
        let timestamp = now.timestamp() as u32;
        let nanos = now.timestamp_subsec_nanos();

        Self {
            exchange,
            symbol,
            timestamp,
            nanos,
            price: 0.0,
            volume: 0.0,
            side: TradeSide::Buy,
            best_bid: 0.0,
            best_ask: 0.0,
            exchange_specific: None,
        }
    }

    pub fn with_timestamp(mut self, datetime: DateTime<Utc>) -> Self {
        self.timestamp = datetime.timestamp() as u32;
        self.nanos = datetime.timestamp_subsec_nanos();
        self
    }

    pub fn with_timestamp_parts(mut self, timestamp: u32, nanos: u32) -> Self {
        self.timestamp = timestamp;
        self.nanos = nanos;
        self
    }
}

impl UnifiedTradeData {
    pub fn new(exchange: ExchangeId, symbol: String, trade_id: u64) -> Self {
        let now = Utc::now();
        let timestamp = now.timestamp() as u32;
        let nanos = now.timestamp_subsec_nanos();

        Self {
            exchange,
            symbol,
            trade_id,
            timestamp,
            nanos,
            price: 0.0,
            size: 0.0,
            side: TradeSide::Buy,
            maker_order_id: [0; 16],
            taker_order_id: [0; 16],
            exchange_specific: None,
        }
    }

    pub fn with_timestamp(mut self, datetime: DateTime<Utc>) -> Self {
        self.timestamp = datetime.timestamp() as u32;
        self.nanos = datetime.timestamp_subsec_nanos();
        self
    }

    pub fn with_timestamp_parts(mut self, timestamp: u32, nanos: u32) -> Self {
        self.timestamp = timestamp;
        self.nanos = nanos;
        self
    }

    pub fn set_maker_order_id(&mut self, id: &str) {
        if let Ok(uuid) = Uuid::parse_str(id) {
            self.maker_order_id = *uuid.as_bytes();
        }
    }

    pub fn set_taker_order_id(&mut self, id: &str) {
        if let Ok(uuid) = Uuid::parse_str(id) {
            self.taker_order_id = *uuid.as_bytes();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_side_conversion() {
        assert_eq!(TradeSide::parse("buy"), Some(TradeSide::Buy));
        assert_eq!(TradeSide::parse("SELL"), Some(TradeSide::Sell));
        assert_eq!(TradeSide::parse("invalid"), None);
        assert_eq!(TradeSide::Buy.as_u32(), 0);
        assert_eq!(TradeSide::Sell.as_u32(), 1);
    }

    #[test]
    fn test_exchange_id_string() {
        assert_eq!(ExchangeId::Coinbase.as_str(), "coinbase");
        assert_eq!(ExchangeId::Binance.as_str(), "binance");
    }

    #[test]
    fn test_unified_data_creation() {
        let market_data = UnifiedMarketData::new(ExchangeId::Coinbase, "BTC-USD".to_string());
        assert_eq!(market_data.exchange, ExchangeId::Coinbase);
        assert_eq!(market_data.symbol, "BTC-USD");
        assert!(market_data.timestamp > 0);

        let trade_data = UnifiedTradeData::new(ExchangeId::Binance, "BTC-USDT".to_string(), 12345);
        assert_eq!(trade_data.exchange, ExchangeId::Binance);
        assert_eq!(trade_data.symbol, "BTC-USDT");
        assert_eq!(trade_data.trade_id, 12345);
    }

    #[test]
    fn test_uuid_conversion() {
        let mut trade = UnifiedTradeData::new(ExchangeId::Coinbase, "BTC-USD".to_string(), 1);
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        trade.set_maker_order_id(uuid_str);

        // Verify the UUID was properly converted to bytes
        assert_ne!(trade.maker_order_id, [0; 16]);
    }
}
