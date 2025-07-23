//! Common data types used across the system

use serde::{Deserialize, Serialize};

/// Time series data point
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: u32,
    pub value: f32,
}

/// OHLC data for candlestick charts
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OhlcData {
    pub timestamp: u32,
    pub open: f32,
    pub high: f32,
    pub low: f32,
    pub close: f32,
    pub volume: Option<f32>,
}

/// Trade data
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TradeData {
    pub timestamp: u32,
    pub price: f32,
    pub volume: f32,
    pub side: TradeSide,
}

/// Trade side (buy/sell)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeSide {
    Buy,
    Sell,
}

/// Data column type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    Time,
    BestBid,
    BestAsk,
    Price,
    Volume,
    Side,
    Open,
    High,
    Low,
    Close,
}

impl ColumnType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ColumnType::Time => "time",
            ColumnType::BestBid => "best_bid",
            ColumnType::BestAsk => "best_ask",
            ColumnType::Price => "price",
            ColumnType::Volume => "volume",
            ColumnType::Side => "side",
            ColumnType::Open => "open",
            ColumnType::High => "high",
            ColumnType::Low => "low",
            ColumnType::Close => "close",
        }
    }
}

/// Data request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRequest {
    pub symbol: String,
    pub data_type: String,
    pub start_time: u64,
    pub end_time: u64,
    pub columns: Vec<String>,
}

/// Data response header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataResponseHeader {
    pub symbol: String,
    pub columns: Vec<String>,
    pub start_time: u64,
    pub end_time: u64,
    pub row_count: usize,
}