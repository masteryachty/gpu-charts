use crate::common::{
    data_types::{ExchangeId, TradeSide, UnifiedMarketData, UnifiedTradeData},
    symbol_mapper::SymbolMapper,
    utils::current_timestamp,
};
use anyhow::Result;
use serde_json::Value;
use tracing::warn;

/// Parse Kraken ticker data from array format
/// Format: [channelID, data, "ticker", pair]
/// Where data is an object with fields like: a, b, c, v, p, t, l, h, o
pub fn parse_kraken_ticker_array(
    data: &Value,
    pair: &str,
    mapper: &SymbolMapper,
) -> Result<Option<UnifiedMarketData>> {
    let obj = data
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Ticker data is not an object"))?;

    let normalized_symbol = mapper
        .normalize(ExchangeId::Kraken, pair)
        .unwrap_or_else(|| crate::common::utils::normalize_symbol_kraken(pair));

    let mut market_data = UnifiedMarketData::new(ExchangeId::Kraken, normalized_symbol);

    // Set current timestamp as Kraken ticker doesn't include timestamp
    let (timestamp, nanos) = current_timestamp();
    market_data.timestamp = timestamp;
    market_data.nanos = nanos;

    // Parse last trade price and volume
    if let Some(c_arr) = obj.get("c").and_then(|v| v.as_array()) {
        if let Some(price_str) = c_arr.first().and_then(|v| v.as_str()) {
            market_data.price = price_str.parse().unwrap_or_else(|e| {
                warn!("Failed to parse Kraken price '{}': {}", price_str, e);
                0.0
            });
        }
        if let Some(volume_str) = c_arr.get(1).and_then(|v| v.as_str()) {
            market_data.volume = volume_str.parse().unwrap_or_else(|e| {
                warn!("Failed to parse Kraken volume '{}': {}", volume_str, e);
                0.0
            });
        }
    }

    // Parse best ask
    if let Some(a_arr) = obj.get("a").and_then(|v| v.as_array()) {
        if let Some(ask_str) = a_arr.first().and_then(|v| v.as_str()) {
            market_data.best_ask = ask_str.parse().unwrap_or_else(|e| {
                warn!("Failed to parse Kraken ask price '{}': {}", ask_str, e);
                0.0
            });
        }
    }

    // Parse best bid
    if let Some(b_arr) = obj.get("b").and_then(|v| v.as_array()) {
        if let Some(bid_str) = b_arr.first().and_then(|v| v.as_str()) {
            market_data.best_bid = bid_str.parse().unwrap_or_else(|e| {
                warn!("Failed to parse Kraken bid price '{}': {}", bid_str, e);
                0.0
            });
        }
    }

    // Determine side based on price movement (compare with opening price)
    market_data.side = if let (Some(c_arr), Some(o_str)) = (
        obj.get("c").and_then(|v| v.as_array()),
        obj.get("o").and_then(|v| v.as_str()),
    ) {
        if let (Some(price_str), Ok(open)) =
            (c_arr.first().and_then(|v| v.as_str()), o_str.parse::<f32>())
        {
            if let Ok(price) = price_str.parse::<f32>() {
                if price >= open {
                    TradeSide::Buy
                } else {
                    TradeSide::Sell
                }
            } else {
                TradeSide::Buy
            }
        } else {
            TradeSide::Buy
        }
    } else {
        TradeSide::Buy
    };

    Ok(Some(market_data))
}

/// Parse Kraken trade data from array format
/// Format: [price, volume, time, side, orderType, misc]
pub fn parse_kraken_trade_array(
    trade: &Value,
    pair: &str,
    mapper: &SymbolMapper,
) -> Result<Option<UnifiedTradeData>> {
    let arr = trade
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Trade data is not an array"))?;

    if arr.len() < 4 {
        return Ok(None);
    }

    let normalized_symbol = mapper
        .normalize(ExchangeId::Kraken, pair)
        .unwrap_or_else(|| crate::common::utils::normalize_symbol_kraken(pair));

    // Use timestamp as trade ID (converted to u64)
    let trade_time = arr[2]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let trade_id = (trade_time * 1_000_000.0) as u64; // Convert to microseconds for unique ID

    let mut trade_data = UnifiedTradeData::new(ExchangeId::Kraken, normalized_symbol, trade_id);

    // Parse timestamp (seconds with decimal)
    let timestamp_secs = trade_time as u32;
    let nanos = ((trade_time - timestamp_secs as f64) * 1_000_000_000.0) as u32;
    trade_data.timestamp = timestamp_secs;
    trade_data.nanos = nanos;

    // Parse price
    if let Some(price_str) = arr[0].as_str() {
        trade_data.price = price_str.parse().unwrap_or_else(|e| {
            warn!("Failed to parse Kraken trade price '{}': {}", price_str, e);
            0.0
        });
    }

    // Parse volume
    if let Some(volume_str) = arr[1].as_str() {
        trade_data.size = volume_str.parse().unwrap_or_else(|e| {
            warn!(
                "Failed to parse Kraken trade volume '{}': {}",
                volume_str, e
            );
            0.0
        });
    }

    // Parse side (b = buy, s = sell)
    trade_data.side = match arr[3].as_str() {
        Some("b") => TradeSide::Buy,
        Some("s") => TradeSide::Sell,
        _ => TradeSide::Buy,
    };

    // Kraken doesn't provide order IDs in trade feed, so we leave them as default

    Ok(Some(trade_data))
}

/// Parse Kraken ticker from object format (used in REST API responses)
pub fn parse_kraken_ticker(
    _value: &Value,
    _mapper: &SymbolMapper,
) -> Result<Option<UnifiedMarketData>> {
    // This would be used if we receive ticker data in a different format
    // For now, we only handle the array format from WebSocket
    Ok(None)
}

/// Parse Kraken trade from object format (used in REST API responses)
pub fn parse_kraken_trade(
    _value: &Value,
    _mapper: &SymbolMapper,
) -> Result<Option<UnifiedTradeData>> {
    // This would be used if we receive trade data in a different format
    // For now, we only handle the array format from WebSocket
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AssetGroup, EquivalenceRules, SymbolMappingsConfig};
    use serde_json::json;

    fn create_test_mapper() -> SymbolMapper {
        let config = SymbolMappingsConfig {
            mappings_file: None,
            auto_discover: true,
            equivalence_rules: EquivalenceRules {
                quote_assets: vec![AssetGroup {
                    group: "USD_EQUIVALENT".to_string(),
                    members: vec!["USD".to_string()],
                    primary: "USD".to_string(),
                }],
            },
        };

        SymbolMapper::new(config).unwrap()
    }

    #[test]
    fn test_parse_ticker() {
        let mapper = create_test_mapper();

        let ticker_data = json!({
            "a": ["50001.00", "1", "1.000"],
            "b": ["49999.00", "1", "1.000"],
            "c": ["50000.00", "0.10000000"],
            "v": ["2000.00000000", "3000.00000000"],
            "p": ["49500.00", "49600.00"],
            "t": [1000, 1500],
            "l": ["49000.00", "49100.00"],
            "h": ["51000.00", "50900.00"],
            "o": "49000.00"
        });

        let result = parse_kraken_ticker_array(&ticker_data, "XBT/USD", &mapper)
            .unwrap()
            .unwrap();

        assert_eq!(result.exchange, ExchangeId::Kraken);
        assert_eq!(result.symbol, "BTC-USD"); // XBT normalized to BTC
        assert_eq!(result.price, 50000.0);
        assert_eq!(result.volume, 0.1);
        assert_eq!(result.best_bid, 49999.0);
        assert_eq!(result.best_ask, 50001.0);
        assert_eq!(result.side, TradeSide::Buy); // Price > open
    }

    #[test]
    fn test_parse_trade() {
        let mapper = create_test_mapper();

        let trade_arr = json!(["3000.00", "0.50000000", "1612345678.123456", "s", "l", ""]);

        let result = parse_kraken_trade_array(&trade_arr, "ETH/USD", &mapper)
            .unwrap()
            .unwrap();

        assert_eq!(result.exchange, ExchangeId::Kraken);
        assert_eq!(result.symbol, "ETH-USD");
        assert_eq!(result.price, 3000.0);
        assert_eq!(result.size, 0.5);
        assert_eq!(result.side, TradeSide::Sell);
        assert_eq!(result.timestamp, 1612345678);
        assert!(result.nanos > 0); // Should have nanosecond precision
    }

    #[test]
    fn test_xbt_to_btc_normalization() {
        let mapper = create_test_mapper();

        let ticker_data = json!({
            "c": ["60000.00", "0.01"],
            "a": ["60001.00", "1", "1.000"],
            "b": ["59999.00", "1", "1.000"],
            "o": "59000.00"
        });

        let result = parse_kraken_ticker_array(&ticker_data, "XBT/EUR", &mapper)
            .unwrap()
            .unwrap();

        assert_eq!(result.symbol, "BTC-EUR"); // XBT should be normalized to BTC
    }
}
