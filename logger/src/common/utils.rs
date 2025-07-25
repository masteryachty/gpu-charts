use chrono::{DateTime, Utc};
use std::time::Duration;
use uuid::Uuid;

pub fn current_timestamp() -> (u32, u32) {
    let now = Utc::now();
    let timestamp = now.timestamp() as u32;
    let nanos = now.timestamp_subsec_nanos();
    (timestamp, nanos)
}

pub fn datetime_to_timestamp(dt: DateTime<Utc>) -> (u32, u32) {
    let timestamp = dt.timestamp() as u32;
    let nanos = dt.timestamp_subsec_nanos();
    (timestamp, nanos)
}

pub fn parse_timestamp_millis(millis: u64) -> (u32, u32) {
    let seconds = millis / 1000;
    let nanos = ((millis % 1000) * 1_000_000) as u32;
    (seconds as u32, nanos)
}

pub fn parse_timestamp_micros(micros: u64) -> (u32, u32) {
    let seconds = micros / 1_000_000;
    let nanos = ((micros % 1_000_000) * 1_000) as u32;
    (seconds as u32, nanos)
}

pub fn uuid_to_bytes(uuid_str: &str) -> Result<[u8; 16], uuid::Error> {
    let uuid = Uuid::parse_str(uuid_str)?;
    Ok(*uuid.as_bytes())
}

pub fn bytes_to_uuid(bytes: &[u8; 16]) -> Uuid {
    Uuid::from_bytes(*bytes)
}

pub fn exponential_backoff(attempt: u32, max_delay: Duration) -> Duration {
    let delay = Duration::from_secs(2u64.pow(attempt.min(10)));
    delay.min(max_delay)
}

pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

pub fn normalize_symbol_coinbase(symbol: &str) -> String {
    // Coinbase uses format like "BTC-USD"
    symbol.to_uppercase()
}

pub fn normalize_symbol_binance(symbol: &str) -> String {
    // Binance uses format like "BTCUSDT"
    // Convert to normalized format "BTC-USDT"
    let symbol = symbol.to_uppercase();

    // Common quote currencies
    let quotes = ["USDT", "USDC", "BUSD", "BTC", "ETH", "BNB", "USD", "EUR"];

    for quote in quotes {
        if symbol.ends_with(quote) {
            let base = &symbol[..symbol.len() - quote.len()];
            return format!("{}-{}", base, quote);
        }
    }

    // If no known quote currency found, return as-is
    symbol
}

pub fn denormalize_symbol_binance(normalized: &str) -> String {
    // Convert from "BTC-USDT" to "BTCUSDT"
    normalized.replace('-', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_parsing() {
        let millis = 1609459200000u64; // 2021-01-01 00:00:00 UTC
        let (seconds, nanos) = parse_timestamp_millis(millis);
        assert_eq!(seconds, 1609459200);
        assert_eq!(nanos, 0);

        let millis_with_fraction = 1609459200123u64;
        let (seconds, nanos) = parse_timestamp_millis(millis_with_fraction);
        assert_eq!(seconds, 1609459200);
        assert_eq!(nanos, 123_000_000);
    }

    #[test]
    fn test_uuid_conversion() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let bytes = uuid_to_bytes(uuid_str).unwrap();
        let uuid = bytes_to_uuid(&bytes);
        assert_eq!(uuid.to_string(), uuid_str);
    }

    #[test]
    fn test_exponential_backoff() {
        let max = Duration::from_secs(60);

        assert_eq!(exponential_backoff(0, max), Duration::from_secs(1));
        assert_eq!(exponential_backoff(1, max), Duration::from_secs(2));
        assert_eq!(exponential_backoff(2, max), Duration::from_secs(4));
        assert_eq!(exponential_backoff(10, max), Duration::from_secs(60)); // capped at max
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_symbol_normalization() {
        assert_eq!(normalize_symbol_coinbase("btc-usd"), "BTC-USD");
        assert_eq!(normalize_symbol_coinbase("ETH-USDT"), "ETH-USDT");

        assert_eq!(normalize_symbol_binance("BTCUSDT"), "BTC-USDT");
        assert_eq!(normalize_symbol_binance("ETHBUSD"), "ETH-BUSD");
        assert_eq!(normalize_symbol_binance("BNBBTC"), "BNB-BTC");

        assert_eq!(denormalize_symbol_binance("BTC-USDT"), "BTCUSDT");
        assert_eq!(denormalize_symbol_binance("ETH-BUSD"), "ETHBUSD");
    }
}
