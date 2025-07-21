#[derive(Clone, Debug, PartialEq)]
pub struct TickerData {
    pub timestamp_secs: u32,
    pub timestamp_nanos: u32,
    pub price: f32,
    pub volume: f32,
    pub side: u8,
    pub best_bid: f32,
    pub best_ask: f32,
}

impl TickerData {
    pub fn new(
        timestamp_secs: u32,
        timestamp_nanos: u32,
        price: f32,
        volume: f32,
        side: u8,
        best_bid: f32,
        best_ask: f32,
    ) -> Self {
        Self {
            timestamp_secs,
            timestamp_nanos,
            price,
            volume,
            side,
            best_bid,
            best_ask,
        }
    }
}

/// Trade-specific data extracted from ticker messages for enhanced trade visualization
#[derive(Clone, Debug, PartialEq)]
pub struct TickerTradeData {
    pub timestamp_secs: u32,
    pub timestamp_nanos: u32,
    pub trade_price: f32,
    pub trade_volume: f32,
    pub trade_side: u8, // 1 = buy, 0 = sell
    pub spread: f32,    // best_ask - best_bid at time of trade
}

impl TickerTradeData {
    /// Create TickerTradeData from TickerData
    pub fn from_ticker(ticker: &TickerData) -> Self {
        Self {
            timestamp_secs: ticker.timestamp_secs,
            timestamp_nanos: ticker.timestamp_nanos,
            trade_price: ticker.price,
            trade_volume: ticker.volume,
            trade_side: ticker.side,
            spread: ticker.best_ask - ticker.best_bid,
        }
    }

    /// Validate trade data
    pub fn is_valid(&self) -> bool {
        self.trade_price > 0.0
            && self.trade_volume > 0.0
            && (self.trade_side == 0 || self.trade_side == 1)
            && self.spread >= 0.0
    }
}

/// Market trade data from market_trades channel
#[derive(Clone, Debug, PartialEq)]
pub struct MarketTradeData {
    pub trade_id: u64,
    pub timestamp_secs: u32,
    pub timestamp_nanos: u32,
    pub price: f32,
    pub size: f32,
    pub side: u8,                 // 1 = buy, 0 = sell
    pub maker_order_id: [u8; 16], // UUID as bytes
    pub taker_order_id: [u8; 16], // UUID as bytes
}

impl MarketTradeData {
    /// Validate market trade data
    pub fn is_valid(&self) -> bool {
        self.trade_id > 0
            && self.price > 0.0
            && self.size > 0.0
            && (self.side == 0 || self.side == 1)
    }
}

/// Helper function to convert UUID string to bytes
pub fn uuid_to_bytes(uuid_str: &str) -> Result<[u8; 16], Box<dyn std::error::Error>> {
    // Remove hyphens and decode hex
    let clean = uuid_str.replace('-', "");
    if clean.len() != 32 {
        return Err("Invalid UUID length".into());
    }

    let mut bytes = [0u8; 16];
    for i in 0..16 {
        let byte_str = &clean[i * 2..i * 2 + 2];
        bytes[i] = u8::from_str_radix(byte_str, 16)?;
    }

    Ok(bytes)
}
