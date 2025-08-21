use crate::common::{
    data_types::{ExchangeId, TradeSide, UnifiedMarketData, UnifiedTradeData},
    utils::{current_timestamp, parse_timestamp_millis},
};
use anyhow::Result;
use serde_json::Value;
use tracing::debug;

pub fn parse_bitfinex_ticker(_value: &Value) -> Result<Option<UnifiedMarketData>> {
    // This function is for REST API responses, not WebSocket
    // Bitfinex REST API ticker format would be handled here if needed
    Ok(None)
}

pub fn parse_bitfinex_trade(_value: &Value) -> Result<Option<UnifiedTradeData>> {
    // This function is for REST API responses, not WebSocket
    // Bitfinex REST API trade format would be handled here if needed
    Ok(None)
}

pub fn parse_bitfinex_ticker_update(
    value: &Value,
    symbol: &str,
) -> Result<Option<UnifiedMarketData>> {
    let arr = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Expected array"))?;

    if arr.len() < 2 {
        return Ok(None);
    }

    // Skip heartbeat messages
    if arr.len() == 2 && arr[1].as_str() == Some("hb") {
        return Ok(None);
    }

    // Ticker format: [CHANNEL_ID, [BID, BID_SIZE, ASK, ASK_SIZE, DAILY_CHANGE, DAILY_CHANGE_PERC, LAST_PRICE, VOLUME, HIGH, LOW]]
    if let Some(ticker_data) = arr.get(1).and_then(|v| v.as_array()) {
        if ticker_data.len() < 10 {
            return Ok(None);
        }

        let mut data = UnifiedMarketData::new(ExchangeId::Bitfinex, symbol.to_string());

        // Use current timestamp as Bitfinex ticker updates don't include timestamp
        let (timestamp, nanos) = current_timestamp();
        data = data.with_timestamp_parts(timestamp, nanos);

        // Parse ticker data with safer null handling
        data.best_bid = ticker_data.get(0)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_f64()
                }
            })
            .unwrap_or_else(|| {
                debug!("Failed to parse Bitfinex bid price from ticker (null or missing)");
                0.0
            }) as f32;
            
        data.best_ask = ticker_data.get(2)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_f64()
                }
            })
            .unwrap_or_else(|| {
                debug!("Failed to parse Bitfinex ask price from ticker (null or missing)");
                0.0
            }) as f32;
            
        data.price = ticker_data.get(6)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_f64()
                }
            })
            .unwrap_or_else(|| {
                debug!("Failed to parse Bitfinex last price from ticker (null or missing)");
                0.0
            }) as f32;
            
        data.volume = ticker_data.get(7)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_f64()
                }
            })
            .unwrap_or_else(|| {
                debug!("Failed to parse Bitfinex volume from ticker (null or missing)");
                0.0
            }) as f32;

        // Determine side based on daily change
        let daily_change = ticker_data.get(4)
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_f64()
                }
            })
            .unwrap_or_else(|| {
                debug!("Failed to parse Bitfinex daily change from ticker (null or missing)");
                0.0
            });
        data.side = if daily_change >= 0.0 {
            TradeSide::Buy
        } else {
            TradeSide::Sell
        };

        Ok(Some(data))
    } else {
        Ok(None)
    }
}

pub fn parse_bitfinex_trade_update(
    value: &Value,
    symbol: &str,
) -> Result<Option<Vec<UnifiedTradeData>>> {
    let arr = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Expected array"))?;

    if arr.len() < 2 {
        return Ok(None);
    }

    // Skip heartbeat messages
    if arr.len() == 2 && arr[1].as_str() == Some("hb") {
        return Ok(None);
    }

    let mut trades = Vec::new();

    match &arr[1] {
        // Snapshot format: [CHANNEL_ID, [[ID, MTS, AMOUNT, PRICE], ...]]
        Value::Array(trade_list) => {
            for trade_item in trade_list {
                if let Some(trade_data) = trade_item.as_array() {
                    if trade_data.len() >= 4 {
                        if let Some(trade) = parse_single_trade(trade_data, symbol)? {
                            trades.push(trade);
                        }
                    }
                }
            }
        }
        // Update format: [CHANNEL_ID, "te" or "tu", [ID, MTS, AMOUNT, PRICE]]
        Value::String(event_type) if event_type == "te" || event_type == "tu" => {
            if let Some(trade_data) = arr.get(2).and_then(|v| v.as_array()) {
                if let Some(trade) = parse_single_trade(trade_data, symbol)? {
                    trades.push(trade);
                }
            }
        }
        _ => return Ok(None),
    }

    if trades.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trades))
    }
}

fn parse_single_trade(trade_data: &[Value], symbol: &str) -> Result<Option<UnifiedTradeData>> {
    if trade_data.len() < 4 {
        return Ok(None);
    }

    let trade_id = trade_data.get(0)
        .and_then(|v| {
            if v.is_null() {
                None
            } else {
                v.as_i64()
            }
        })
        .unwrap_or_else(|| {
            debug!("Failed to parse Bitfinex trade ID (null or missing)");
            0
        }) as u64;
        
    let timestamp_ms = trade_data.get(1)
        .and_then(|v| {
            if v.is_null() {
                None
            } else {
                v.as_i64()
            }
        })
        .unwrap_or_else(|| {
            debug!("Failed to parse Bitfinex trade timestamp (null or missing)");
            0
        }) as u64;
        
    let amount = trade_data.get(2)
        .and_then(|v| {
            if v.is_null() {
                None
            } else {
                v.as_f64()
            }
        })
        .unwrap_or_else(|| {
            debug!("Failed to parse Bitfinex trade amount (null or missing)");
            0.0
        });
        
    let price = trade_data.get(3)
        .and_then(|v| {
            if v.is_null() {
                None
            } else {
                v.as_f64()
            }
        })
        .unwrap_or_else(|| {
            debug!("Failed to parse Bitfinex trade price (null or missing)");
            0.0
        });

    let mut data = UnifiedTradeData::new(ExchangeId::Bitfinex, symbol.to_string(), trade_id);

    // Parse timestamp
    let (timestamp, nanos) = parse_timestamp_millis(timestamp_ms);
    data = data.with_timestamp_parts(timestamp, nanos);

    data.price = price as f32;
    data.size = amount.abs() as f32;

    // Negative amount means sell, positive means buy
    data.side = if amount >= 0.0 {
        TradeSide::Buy
    } else {
        TradeSide::Sell
    };

    Ok(Some(data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_ticker_update() {
        let ticker_json = json!([
            123, // channel ID
            [
                50000.0, // BID
                0.5,     // BID_SIZE
                50001.0, // ASK
                0.4,     // ASK_SIZE
                1000.0,  // DAILY_CHANGE
                2.0,     // DAILY_CHANGE_PERC
                50000.5, // LAST_PRICE
                1234.56, // VOLUME
                51000.0, // HIGH
                49000.0  // LOW
            ]
        ]);

        let result = parse_bitfinex_ticker_update(&ticker_json, "tBTCUSD")
            .unwrap()
            .unwrap();

        assert_eq!(result.exchange, ExchangeId::Bitfinex);
        assert_eq!(result.symbol, "tBTCUSD");
        assert_eq!(result.best_bid, 50000.0);
        assert_eq!(result.best_ask, 50001.0);
        assert_eq!(result.price, 50000.5);
        assert_eq!(result.volume, 1234.56);
        assert_eq!(result.side, TradeSide::Buy);
    }

    #[test]
    fn test_parse_trade_update_single() {
        let trade_json = json!([
            234,  // channel ID
            "te", // trade executed
            [
                123456789,        // ID
                1640995200000i64, // MTS (milliseconds)
                0.1,              // AMOUNT (positive = buy)
                30000.0           // PRICE
            ]
        ]);

        let trades = parse_bitfinex_trade_update(&trade_json, "tETHUSD")
            .unwrap()
            .unwrap();

        assert_eq!(trades.len(), 1);
        let trade = &trades[0];
        assert_eq!(trade.exchange, ExchangeId::Bitfinex);
        assert_eq!(trade.symbol, "tETHUSD");
        assert_eq!(trade.trade_id, 123456789);
        assert_eq!(trade.price, 30000.0);
        assert_eq!(trade.size, 0.1);
        assert_eq!(trade.side, TradeSide::Buy);
    }

    #[test]
    fn test_parse_trade_update_sell() {
        let trade_json = json!([
            234,  // channel ID
            "te", // trade executed
            [
                987654321,        // ID
                1640995200000i64, // MTS (milliseconds)
                -0.5,             // AMOUNT (negative = sell)
                30000.0           // PRICE
            ]
        ]);

        let trades = parse_bitfinex_trade_update(&trade_json, "tETHUSD")
            .unwrap()
            .unwrap();

        assert_eq!(trades.len(), 1);
        let trade = &trades[0];
        assert_eq!(trade.side, TradeSide::Sell);
        assert_eq!(trade.size, 0.5); // Size is absolute value
    }

    #[test]
    fn test_parse_trade_snapshot() {
        let snapshot_json = json!([
            234, // channel ID
            [
                [123456789, 1640995200000i64, 0.1, 30000.0],
                [123456790, 1640995201000i64, -0.2, 30001.0],
                [123456791, 1640995202000i64, 0.15, 29999.0]
            ]
        ]);

        let trades = parse_bitfinex_trade_update(&snapshot_json, "tETHUSD")
            .unwrap()
            .unwrap();

        assert_eq!(trades.len(), 3);
        assert_eq!(trades[0].trade_id, 123456789);
        assert_eq!(trades[0].side, TradeSide::Buy);
        assert_eq!(trades[1].trade_id, 123456790);
        assert_eq!(trades[1].side, TradeSide::Sell);
        assert_eq!(trades[2].trade_id, 123456791);
        assert_eq!(trades[2].side, TradeSide::Buy);
    }

    #[test]
    fn test_parse_heartbeat() {
        let hb_json = json!([123, "hb"]);

        let ticker_result = parse_bitfinex_ticker_update(&hb_json, "tBTCUSD").unwrap();
        assert!(ticker_result.is_none());

        let trade_result = parse_bitfinex_trade_update(&hb_json, "tBTCUSD").unwrap();
        assert!(trade_result.is_none());
    }
}
