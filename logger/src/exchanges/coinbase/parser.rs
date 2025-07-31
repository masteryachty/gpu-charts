use crate::common::data_types::{ExchangeId, TradeSide, UnifiedMarketData, UnifiedTradeData};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::warn;

pub fn parse_coinbase_ticker(value: &Value) -> Result<Option<UnifiedMarketData>> {
    if value["type"].as_str() != Some("ticker") {
        return Ok(None);
    }

    let product_id = value["product_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing product_id"))?;

    let normalized_symbol = product_id.to_string();

    let mut data = UnifiedMarketData::new(ExchangeId::Coinbase, normalized_symbol);

    // Parse timestamp
    if let Some(time_str) = value["time"].as_str() {
        if let Ok(dt) = DateTime::parse_from_rfc3339(time_str) {
            data = data.with_timestamp(dt.with_timezone(&Utc));
        }
    }

    // Parse price and volume from last trade
    if let Some(price_str) = value["price"].as_str() {
        data.price = price_str.parse().unwrap_or_else(|e| {
            warn!("Failed to parse Coinbase price '{}': {}", price_str, e);
            0.0
        });
    }

    if let Some(volume_str) = value["last_size"].as_str() {
        data.volume = volume_str.parse().unwrap_or_else(|e| {
            warn!("Failed to parse Coinbase volume '{}': {}", volume_str, e);
            0.0
        });
    }

    // Parse best bid/ask
    if let Some(bid_str) = value["best_bid"].as_str() {
        data.best_bid = bid_str.parse().unwrap_or_else(|e| {
            warn!("Failed to parse Coinbase bid price '{}': {}", bid_str, e);
            0.0
        });
    }

    if let Some(ask_str) = value["best_ask"].as_str() {
        data.best_ask = ask_str.parse().unwrap_or_else(|e| {
            warn!("Failed to parse Coinbase ask price '{}': {}", ask_str, e);
            0.0
        });
    }

    // Determine side from price movement or default to buy
    data.side = if let (Some(price_str), Some(open_str)) =
        (value["price"].as_str(), value["open_24h"].as_str())
    {
        if let (Ok(price), Ok(open)) = (price_str.parse::<f32>(), open_str.parse::<f32>()) {
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
    };

    Ok(Some(data))
}

pub fn parse_coinbase_trade(value: &Value) -> Result<Option<UnifiedTradeData>> {
    if value["type"].as_str() != Some("match") && value["type"].as_str() != Some("last_match") {
        return Ok(None);
    }

    let product_id = value["product_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing product_id"))?;

    let normalized_symbol = product_id.to_string();

    let trade_id = value["trade_id"]
        .as_u64()
        .or_else(|| value["sequence"].as_u64())
        .unwrap_or(0);

    let mut data = UnifiedTradeData::new(ExchangeId::Coinbase, normalized_symbol, trade_id);

    // Parse timestamp
    if let Some(time_str) = value["time"].as_str() {
        if let Ok(dt) = DateTime::parse_from_rfc3339(time_str) {
            data = data.with_timestamp(dt.with_timezone(&Utc));
        }
    }

    // Parse price and size
    if let Some(price_str) = value["price"].as_str() {
        data.price = price_str.parse().unwrap_or_else(|e| {
            warn!(
                "Failed to parse Coinbase trade price '{}': {}",
                price_str, e
            );
            0.0
        });
    }

    if let Some(size_str) = value["size"].as_str() {
        data.size = size_str.parse().unwrap_or_else(|e| {
            warn!("Failed to parse Coinbase trade size '{}': {}", size_str, e);
            0.0
        });
    }

    // Parse side
    data.side = match value["side"].as_str() {
        Some("buy") => TradeSide::Buy,
        Some("sell") => TradeSide::Sell,
        _ => TradeSide::Buy,
    };

    // Parse order IDs
    if let Some(maker_id) = value["maker_order_id"].as_str() {
        data.set_maker_order_id(maker_id);
    }

    if let Some(taker_id) = value["taker_order_id"].as_str() {
        data.set_taker_order_id(taker_id);
    }

    Ok(Some(data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_ticker() {
        let ticker_json = json!({
            "type": "ticker",
            "product_id": "BTC-USD",
            "price": "50000.00",
            "last_size": "0.1",
            "best_bid": "49999.00",
            "best_ask": "50001.00",
            "time": "2023-01-01T00:00:00.000Z",
            "open_24h": "49000.00"
        });

        let result = parse_coinbase_ticker(&ticker_json).unwrap().unwrap();

        assert_eq!(result.exchange, ExchangeId::Coinbase);
        assert_eq!(result.symbol, "BTC-USD");
        assert_eq!(result.price, 50000.0);
        assert_eq!(result.volume, 0.1);
        assert_eq!(result.best_bid, 49999.0);
        assert_eq!(result.best_ask, 50001.0);
        assert_eq!(result.side, TradeSide::Buy);
    }

    #[test]
    fn test_parse_trade() {
        let trade_json = json!({
            "type": "match",
            "trade_id": 123456,
            "product_id": "ETH-USD",
            "price": "3000.00",
            "size": "0.5",
            "side": "sell",
            "time": "2023-01-01T00:00:00.000Z",
            "maker_order_id": "550e8400-e29b-41d4-a716-446655440000",
            "taker_order_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
        });

        let result = parse_coinbase_trade(&trade_json).unwrap().unwrap();

        assert_eq!(result.exchange, ExchangeId::Coinbase);
        assert_eq!(result.symbol, "ETH-USD");
        assert_eq!(result.trade_id, 123456);
        assert_eq!(result.price, 3000.0);
        assert_eq!(result.size, 0.5);
        assert_eq!(result.side, TradeSide::Sell);
        assert_ne!(result.maker_order_id, [0; 16]);
        assert_ne!(result.taker_order_id, [0; 16]);
    }
}
