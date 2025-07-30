use crate::common::{
    data_types::{ExchangeId, TradeSide, UnifiedMarketData, UnifiedTradeData},
    symbol_mapper::SymbolMapper,
    utils::{normalize_symbol_binance, parse_timestamp_millis},
};
use anyhow::Result;
use serde_json::Value;

pub fn parse_binance_ticker(
    value: &Value,
    mapper: &SymbolMapper,
) -> Result<Option<UnifiedMarketData>> {
    // Check if this is a 24hr ticker event
    if value["e"].as_str() != Some("24hrTicker") {
        return Ok(None);
    }

    let symbol = value["s"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing symbol"))?;

    let normalized_symbol = mapper
        .normalize(ExchangeId::Binance, symbol)
        .unwrap_or_else(|| normalize_symbol_binance(symbol));

    let mut data = UnifiedMarketData::new(ExchangeId::Binance, normalized_symbol);

    // Parse timestamp (Binance uses milliseconds)
    if let Some(event_time) = value["E"].as_u64() {
        let (timestamp, nanos) = parse_timestamp_millis(event_time);
        data.timestamp = timestamp;
        data.nanos = nanos;
    }

    // Parse last price
    if let Some(price_str) = value["c"].as_str() {
        data.price = price_str.parse().unwrap_or(0.0);
    }

    // Parse volume (24hr volume in quote asset)
    if let Some(volume_str) = value["v"].as_str() {
        data.volume = volume_str.parse().unwrap_or(0.0);
    }

    // Parse best bid/ask
    if let Some(bid_str) = value["b"].as_str() {
        data.best_bid = bid_str.parse().unwrap_or(0.0);
    }

    if let Some(ask_str) = value["a"].as_str() {
        data.best_ask = ask_str.parse().unwrap_or(0.0);
    }

    // Determine side based on price change
    data.side = if let (Some(last_str), Some(open_str)) = (value["c"].as_str(), value["o"].as_str())
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

    Ok(Some(data))
}

pub fn parse_binance_trade(
    value: &Value,
    mapper: &SymbolMapper,
) -> Result<Option<UnifiedTradeData>> {
    // Check if this is a trade event
    if value["e"].as_str() != Some("trade") {
        return Ok(None);
    }

    let symbol = value["s"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing symbol"))?;

    let normalized_symbol = mapper
        .normalize(ExchangeId::Binance, symbol)
        .unwrap_or_else(|| normalize_symbol_binance(symbol));

    let trade_id = value["t"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Missing trade ID"))?;

    let mut data = UnifiedTradeData::new(ExchangeId::Binance, normalized_symbol, trade_id);

    // Parse timestamp (Binance uses milliseconds)
    if let Some(trade_time) = value["T"].as_u64() {
        let (timestamp, nanos) = parse_timestamp_millis(trade_time);
        data.timestamp = timestamp;
        data.nanos = nanos;
    }

    // Parse price
    if let Some(price_str) = value["p"].as_str() {
        data.price = price_str.parse().unwrap_or(0.0);
    }

    // Parse quantity
    if let Some(quantity_str) = value["q"].as_str() {
        data.size = quantity_str.parse().unwrap_or(0.0);
    }

    // Parse side (m = true means buyer is maker, so trade is a sell)
    data.side = match value["m"].as_bool() {
        Some(true) => TradeSide::Sell, // Buyer is maker = market sell
        Some(false) => TradeSide::Buy, // Seller is maker = market buy
        None => TradeSide::Buy,
    };

    // Binance doesn't provide order IDs in trade stream
    // We'll use trade ID as a placeholder
    let trade_id_bytes = trade_id.to_le_bytes();
    data.maker_order_id[..8].copy_from_slice(&trade_id_bytes);
    data.taker_order_id[8..16].copy_from_slice(&trade_id_bytes);
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
                    primary: "USD".to_string(),
                }],
            },
        };

        SymbolMapper::new(config).unwrap()
    }

    #[test]
    fn test_parse_binance_ticker() {
        let mapper = create_test_mapper();

        let ticker_json = json!({
            "e": "24hrTicker",
            "E": 1672531200000u64,  // 2023-01-01 00:00:00
            "s": "BTCUSDT",
            "c": "50000.00",        // Last price
            "o": "49000.00",        // Open price
            "h": "51000.00",        // High price
            "l": "48500.00",        // Low price
            "v": "1234.56",         // Volume
            "b": "49999.00",        // Best bid price
            "B": "0.5",             // Best bid quantity
            "a": "50001.00",        // Best ask price
            "A": "0.3"              // Best ask quantity
        });

        let result = parse_binance_ticker(&ticker_json, &mapper)
            .unwrap()
            .unwrap();

        assert_eq!(result.exchange, ExchangeId::Binance);
        assert_eq!(result.symbol, "BTC-USDT");
        assert_eq!(result.price, 50000.0);
        assert_eq!(result.volume, 1234.56);
        assert_eq!(result.best_bid, 49999.0);
        assert_eq!(result.best_ask, 50001.0);
        assert_eq!(result.side, TradeSide::Buy); // Price went up
        assert_eq!(result.timestamp, 1672531200);
        assert_eq!(result.nanos, 0);
    }

    #[test]
    fn test_parse_binance_trade() {
        let mapper = create_test_mapper();

        let trade_json = json!({
            "e": "trade",
            "E": 1672531200123u64,  // Event time
            "s": "ETHUSDT",
            "t": 123456789,         // Trade ID
            "p": "3000.50",         // Price
            "q": "0.5",             // Quantity
            "T": 1672531200100u64,  // Trade time
            "m": true,              // Is buyer the maker?
            "M": true               // Ignore
        });

        let result = parse_binance_trade(&trade_json, &mapper).unwrap().unwrap();

        assert_eq!(result.exchange, ExchangeId::Binance);
        assert_eq!(result.symbol, "ETH-USDT");
        assert_eq!(result.trade_id, 123456789);
        assert_eq!(result.price, 3000.5);
        assert_eq!(result.size, 0.5);
        assert_eq!(result.side, TradeSide::Sell); // m=true means sell
        assert_eq!(result.timestamp, 1672531200);
        assert_eq!(result.nanos, 100_000_000); // 100ms
    }

    #[test]
    fn test_symbol_normalization() {
        let _mapper = create_test_mapper();

        // Test various Binance symbols
        assert_eq!(normalize_symbol_binance("BTCUSDT"), "BTC-USDT");
        assert_eq!(normalize_symbol_binance("ETHBUSD"), "ETH-BUSD");
        assert_eq!(normalize_symbol_binance("BNBBTC"), "BNB-BTC");
    }
}
