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
