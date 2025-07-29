use crate::common::{
    data_types::{ExchangeId, TradeSide, UnifiedMarketData, UnifiedTradeData},
    symbol_mapper::SymbolMapper,
    utils::parse_timestamp_millis,
};
use anyhow::Result;
use serde_json::Value;

pub fn parse_okx_ticker(value: &Value, mapper: &SymbolMapper) -> Result<Option<UnifiedMarketData>> {
    // OKX ticker format:
    // {
    //   "instType": "SPOT",
    //   "instId": "BTC-USDT",
    //   "last": "43508.1",
    //   "lastSz": "0.00001",
    //   "askPx": "43508.1",
    //   "askSz": "0.0001",
    //   "bidPx": "43508",
    //   "bidSz": "0.001",
    //   "open24h": "43000",
    //   "high24h": "44000",
    //   "low24h": "42000",
    //   "volCcy24h": "1234567890.123",
    //   "vol24h": "12345.678",
    //   "ts": "1597026383085"
    // }

    let inst_id = value["instId"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing instId"))?;

    let normalized_symbol = mapper
        .normalize(ExchangeId::OKX, inst_id)
        .unwrap_or_else(|| inst_id.to_string());

    let mut data = UnifiedMarketData::new(ExchangeId::OKX, normalized_symbol);

    // Parse timestamp
    if let Some(ts_str) = value["ts"].as_str() {
        if let Ok(ts_millis) = ts_str.parse::<u64>() {
            let (timestamp, nanos) = parse_timestamp_millis(ts_millis);
            data.timestamp = timestamp;
            data.nanos = nanos;
        }
    }

    // Parse last trade price and size
    if let Some(last_str) = value["last"].as_str() {
        data.price = last_str.parse().unwrap_or(0.0);
    }

    if let Some(last_sz_str) = value["lastSz"].as_str() {
        data.volume = last_sz_str.parse().unwrap_or(0.0);
    }

    // Parse best bid/ask
    if let Some(bid_str) = value["bidPx"].as_str() {
        data.best_bid = bid_str.parse().unwrap_or(0.0);
    }

    if let Some(ask_str) = value["askPx"].as_str() {
        data.best_ask = ask_str.parse().unwrap_or(0.0);
    }

    // Determine side based on price movement
    data.side = if let (Some(last_str), Some(open_str)) =
        (value["last"].as_str(), value["open24h"].as_str())
    {
        if let (Ok(last), Ok(open)) = (last_str.parse::<f32>(), open_str.parse::<f32>()) {
            if last >= open {
                TradeSide::Buy
            } else {
                TradeSide::Sell
            }
        } else {
            TradeSide::Buy
        }
    } else {
        TradeSide::Buy
    };

    // Store additional OKX-specific data
    if let Some(vol24h) = value["vol24h"].as_str() {
        let mut extra = std::collections::HashMap::new();
        extra.insert(
            "vol24h".to_string(),
            serde_json::Value::String(vol24h.to_string()),
        );

        if let Some(vol_ccy) = value["volCcy24h"].as_str() {
            extra.insert(
                "volCcy24h".to_string(),
                serde_json::Value::String(vol_ccy.to_string()),
            );
        }

        data.exchange_specific = Some(extra);
    }

    Ok(Some(data))
}

pub fn parse_okx_trade(value: &Value, mapper: &SymbolMapper) -> Result<Option<UnifiedTradeData>> {
    // OKX trade format:
    // {
    //   "instId": "BTC-USDT",
    //   "tradeId": "242720720",
    //   "px": "43508.1",
    //   "sz": "0.00001",
    //   "side": "buy",
    //   "ts": "1597026383085"
    // }

    let inst_id = value["instId"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing instId"))?;

    let normalized_symbol = mapper
        .normalize(ExchangeId::OKX, inst_id)
        .unwrap_or_else(|| inst_id.to_string());

    let trade_id = value["tradeId"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let mut data = UnifiedTradeData::new(ExchangeId::OKX, normalized_symbol, trade_id);

    // Parse timestamp
    if let Some(ts_str) = value["ts"].as_str() {
        if let Ok(ts_millis) = ts_str.parse::<u64>() {
            let (timestamp, nanos) = parse_timestamp_millis(ts_millis);
            data.timestamp = timestamp;
            data.nanos = nanos;
        }
    }

    // Parse price and size
    if let Some(px_str) = value["px"].as_str() {
        data.price = px_str.parse().unwrap_or(0.0);
    }

    if let Some(sz_str) = value["sz"].as_str() {
        data.size = sz_str.parse().unwrap_or(0.0);
    }

    // Parse side
    data.side = match value["side"].as_str() {
        Some("buy") => TradeSide::Buy,
        Some("sell") => TradeSide::Sell,
        _ => TradeSide::Buy,
    };

    // OKX doesn't provide maker/taker order IDs in public trade data
    // We'll generate synthetic IDs based on trade ID and timestamp
    // Using a simple hash-based approach for deterministic IDs
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    trade_id.hash(&mut hasher);
    data.timestamp.hash(&mut hasher);
    inst_id.hash(&mut hasher);

    let hash = hasher.finish();
    let mut bytes = [0u8; 16];
    bytes[0..8].copy_from_slice(&hash.to_le_bytes());
    bytes[8..16].copy_from_slice(&hash.to_be_bytes());

    data.maker_order_id = bytes;
    data.taker_order_id = bytes;

    Ok(Some(data))
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
                    members: vec!["USDT".to_string()],
                    primary: "USDT".to_string(),
                }],
            },
        };

        SymbolMapper::new(config).unwrap()
    }

    #[test]
    fn test_parse_ticker() {
        let mapper = create_test_mapper();

        let ticker_json = json!({
            "instType": "SPOT",
            "instId": "BTC-USDT",
            "last": "43508.1",
            "lastSz": "0.00001",
            "askPx": "43508.1",
            "askSz": "0.0001",
            "bidPx": "43508",
            "bidSz": "0.001",
            "open24h": "43000",
            "high24h": "44000",
            "low24h": "42000",
            "volCcy24h": "1234567890.123",
            "vol24h": "12345.678",
            "ts": "1609459200000"
        });

        let result = parse_okx_ticker(&ticker_json, &mapper).unwrap().unwrap();

        assert_eq!(result.exchange, ExchangeId::OKX);
        assert_eq!(result.symbol, "BTC-USDT");
        assert_eq!(result.price, 43508.1);
        assert_eq!(result.volume, 0.00001);
        assert_eq!(result.best_bid, 43508.0);
        assert_eq!(result.best_ask, 43508.1);
        assert_eq!(result.side, TradeSide::Buy);
        assert_eq!(result.timestamp, 1609459200);
        assert_eq!(result.nanos, 0);
        assert!(result.exchange_specific.is_some());
    }

    #[test]
    fn test_parse_trade() {
        let mapper = create_test_mapper();

        let trade_json = json!({
            "instId": "BTC-USDT",
            "tradeId": "242720720",
            "px": "43508.1",
            "sz": "0.00001",
            "side": "sell",
            "ts": "1609459200000"
        });

        let result = parse_okx_trade(&trade_json, &mapper).unwrap().unwrap();

        assert_eq!(result.exchange, ExchangeId::OKX);
        assert_eq!(result.symbol, "BTC-USDT");
        assert_eq!(result.trade_id, 242720720);
        assert_eq!(result.price, 43508.1);
        assert_eq!(result.size, 0.00001);
        assert_eq!(result.side, TradeSide::Sell);
        assert_eq!(result.timestamp, 1609459200);
        assert_eq!(result.nanos, 0);
        assert_ne!(result.maker_order_id, [0; 16]);
        assert_ne!(result.taker_order_id, [0; 16]);
    }

    #[test]
    fn test_parse_ticker_with_price_decrease() {
        let mapper = create_test_mapper();

        let ticker_json = json!({
            "instType": "SPOT",
            "instId": "ETH-USDT",
            "last": "2500",
            "lastSz": "0.1",
            "askPx": "2501",
            "askSz": "1",
            "bidPx": "2499",
            "bidSz": "1",
            "open24h": "2600",
            "ts": "1609459200000"
        });

        let result = parse_okx_ticker(&ticker_json, &mapper).unwrap().unwrap();

        assert_eq!(result.side, TradeSide::Sell); // Price decreased from open
    }
}
