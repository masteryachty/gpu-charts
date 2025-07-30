use crate::common::SymbolMapper;
use crate::exchanges::{Channel, ExchangeConnection, Message};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message as WsMessage, MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info, warn};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct BitfinexConnection {
    url: String,
    symbols: Vec<String>,
    data_sender: mpsc::Sender<Message>,
    ws_stream: Option<WsStream>,
    symbol_mapper: Arc<SymbolMapper>,
    _ping_interval_secs: Option<u64>,
    channel_map: Arc<Mutex<HashMap<i64, ChannelInfo>>>,
}

#[derive(Debug, Clone)]
struct ChannelInfo {
    channel: String,
    symbol: String,
}

impl BitfinexConnection {
    pub fn new(
        url: String,
        symbols: Vec<String>,
        data_sender: mpsc::Sender<Message>,
        symbol_mapper: Arc<SymbolMapper>,
        ping_interval_secs: Option<u64>,
    ) -> Self {
        Self {
            url,
            symbols,
            data_sender,
            ws_stream: None,
            symbol_mapper,
            _ping_interval_secs: ping_interval_secs,
            channel_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn clone_for_ping(&self) -> mpsc::Sender<Message> {
        self.data_sender.clone()
    }

    async fn send_json(&mut self, payload: Value) -> Result<()> {
        if let Some(ws) = &mut self.ws_stream {
            let msg = WsMessage::Text(payload.to_string());
            ws.send(msg).await?;
            Ok(())
        } else {
            Err(anyhow!("WebSocket not connected"))
        }
    }

    async fn process_message(&self, text: &str) -> Result<()> {
        let value: Value = serde_json::from_str(text)?;

        // Bitfinex sends both array and object messages
        match value {
            Value::Array(ref arr) => {
                if arr.is_empty() {
                    return Ok(());
                }

                // Channel data format: [CHANNEL_ID, ...]
                if let Some(channel_id) = arr[0].as_i64() {
                    let channel_info = {
                        let map = self.channel_map.lock().unwrap();
                        map.get(&channel_id).cloned()
                    };

                    if let Some(info) = channel_info {
                        // Skip heartbeat messages [CHANNEL_ID, "hb"]
                        if arr.len() == 2 && arr[1].as_str() == Some("hb") {
                            return Ok(());
                        }

                        match info.channel.as_str() {
                            "ticker" => {
                                if let Some(data) = super::parser::parse_bitfinex_ticker_update(
                                    &value,
                                    &info.symbol,
                                    &self.symbol_mapper,
                                )? {
                                    self.data_sender.send(Message::MarketData(data)).await?;
                                }
                            }
                            "trades" => {
                                if let Some(trades) = super::parser::parse_bitfinex_trade_update(
                                    &value,
                                    &info.symbol,
                                    &self.symbol_mapper,
                                )? {
                                    for trade in trades {
                                        self.data_sender.send(Message::Trade(trade)).await?;
                                    }
                                }
                            }
                            _ => {
                                debug!("Unhandled channel type: {}", info.channel);
                            }
                        }
                    }
                }
            }
            Value::Object(ref obj) => {
                // Handle event messages
                if let Some(event) = obj.get("event").and_then(|v| v.as_str()) {
                    match event {
                        "info" => {
                            info!("Bitfinex info: {:?}", obj);
                        }
                        "subscribed" => {
                            if let (Some(chan_id), Some(channel), Some(symbol)) = (
                                obj.get("chanId").and_then(|v| v.as_i64()),
                                obj.get("channel").and_then(|v| v.as_str()),
                                obj.get("symbol").and_then(|v| v.as_str()),
                            ) {
                                let mut map = self.channel_map.lock().unwrap();
                                map.insert(
                                    chan_id,
                                    ChannelInfo {
                                        channel: channel.to_string(),
                                        symbol: symbol.to_string(),
                                    },
                                );
                                debug!(
                                    "Subscribed to {} {} with channel ID {}",
                                    channel, symbol, chan_id
                                );
                            }
                        }
                        "error" => {
                            let code = obj.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
                            let msg = obj
                                .get("msg")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown error");
                            error!("Bitfinex error (code {}): {}", code, msg);
                            self.data_sender
                                .send(Message::Error(format!("Bitfinex error: {msg}")))
                                .await?;
                        }
                        "pong" => {
                            debug!("Received pong from Bitfinex");
                        }
                        _ => {
                            debug!("Unhandled event: {}", event);
                        }
                    }
                }
            }
            _ => {
                debug!("Unexpected message format: {:?}", value);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ExchangeConnection for BitfinexConnection {
    async fn connect(&mut self) -> Result<()> {
        info!("Connecting to Bitfinex WebSocket: {}", self.url);

        let (ws_stream, _) = connect_async(&self.url).await?;
        self.ws_stream = Some(ws_stream);

        info!("Connected to Bitfinex WebSocket");
        Ok(())
    }

    async fn subscribe(&mut self, channels: Vec<Channel>) -> Result<()> {
        // Subscribe to each symbol and channel combination
        let symbols = self.symbols.clone(); // Clone to avoid borrow checker issues
        for symbol in symbols.iter() {
            for channel in &channels {
                let channel_name = match channel {
                    Channel::Ticker => "ticker",
                    Channel::Trades => "trades",
                    Channel::OrderBook => "book",
                };

                let subscribe_msg = json!({
                    "event": "subscribe",
                    "channel": channel_name,
                    "symbol": symbol.clone()
                });

                self.send_json(subscribe_msg).await?;
            }
        }

        info!("Subscribed to {} symbols on Bitfinex", self.symbols.len());
        Ok(())
    }

    async fn read_message(&mut self) -> Result<Option<Value>> {
        if let Some(ws) = &mut self.ws_stream {
            match ws.next().await {
                Some(Ok(WsMessage::Text(text))) => {
                    self.process_message(&text).await?;
                    Ok(Some(serde_json::from_str(&text)?))
                }
                Some(Ok(WsMessage::Ping(data))) => {
                    if let Some(ws) = &mut self.ws_stream {
                        ws.send(WsMessage::Pong(data)).await?;
                    }
                    Ok(None)
                }
                Some(Ok(WsMessage::Close(_))) => {
                    warn!("WebSocket closed by server");
                    Ok(None)
                }
                Some(Err(e)) => {
                    error!("WebSocket error: {}", e);
                    Err(e.into())
                }
                None => {
                    warn!("WebSocket stream ended");
                    Ok(None)
                }
                _ => Ok(None),
            }
        } else {
            Err(anyhow!("WebSocket not connected"))
        }
    }

    async fn send_ping(&mut self) -> Result<()> {
        // Bitfinex uses event-based ping/pong
        let ping_msg = json!({
            "event": "ping",
            "cid": chrono::Utc::now().timestamp_millis()
        });

        self.send_json(ping_msg).await
    }

    async fn reconnect(&mut self) -> Result<()> {
        self.ws_stream = None;
        self.channel_map.lock().unwrap().clear();
        self.connect().await?;
        self.subscribe(vec![Channel::Ticker, Channel::Trades]).await
    }

    fn is_connected(&self) -> bool {
        self.ws_stream.is_some()
    }

    fn symbols(&self) -> &[String] {
        &self.symbols
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_creation() {
        use crate::config::{AssetGroup, EquivalenceRules, SymbolMappingsConfig};

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

        let mapper = Arc::new(crate::common::SymbolMapper::new(config).unwrap());
        let (tx, _rx) = mpsc::channel(100);
        let conn = BitfinexConnection::new(
            "wss://api-pub.bitfinex.com/ws/2".to_string(),
            vec!["tBTCUSD".to_string()],
            tx,
            mapper,
            Some(15),
        );

        assert_eq!(conn.symbols(), &["tBTCUSD"]);
        assert!(!conn.is_connected());
    }
}
