use crate::common::SymbolMapper;
use crate::exchanges::{Channel, ExchangeConnection, Message};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message as WsMessage, MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info, warn};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Clone)]
pub struct BinanceConnection {
    base_url: String,
    symbols: Vec<String>,
    data_sender: mpsc::Sender<Message>,
    ws_stream: Arc<Mutex<Option<WsStream>>>,
    ping_interval_secs: u64,
    symbol_mapper: Arc<SymbolMapper>,
}

impl BinanceConnection {
    pub fn new(
        base_url: String,
        symbols: Vec<String>,
        data_sender: mpsc::Sender<Message>,
        ping_interval_secs: u64,
        symbol_mapper: Arc<SymbolMapper>,
    ) -> Self {
        Self {
            base_url,
            symbols,
            data_sender,
            ws_stream: Arc::new(Mutex::new(None)),
            ping_interval_secs,
            symbol_mapper,
        }
    }

    fn build_stream_url(&self, channels: &[Channel]) -> String {
        let mut streams = Vec::new();

        for symbol in &self.symbols {
            // Convert symbol to lowercase for Binance
            let symbol_lower = symbol.to_lowercase();

            for channel in channels {
                let stream = match channel {
                    Channel::Ticker => format!("{}@ticker", symbol_lower),
                    Channel::Trades => format!("{}@trade", symbol_lower),
                    Channel::OrderBook => format!("{}@depth", symbol_lower),
                };
                streams.push(stream);
            }
        }

        format!("{}/stream?streams={}", self.base_url, streams.join("/"))
    }

    async fn send_json(&self, payload: Value) -> Result<()> {
        let mut stream = self.ws_stream.lock().await;
        if let Some(ws) = &mut *stream {
            let msg = WsMessage::Text(payload.to_string());
            ws.send(msg).await?;
            Ok(())
        } else {
            Err(anyhow!("WebSocket not connected"))
        }
    }

    async fn process_message(&self, text: &str) -> Result<()> {
        let value: Value = serde_json::from_str(text)?;

        // Binance sends data wrapped in a stream object
        if let Some(data) = value.get("data") {
            if let Some(event_type) = data["e"].as_str() {
                match event_type {
                    "24hrTicker" => {
                        if let Some(market_data) =
                            super::parser::parse_binance_ticker(data, &self.symbol_mapper)?
                        {
                            self.data_sender
                                .send(Message::MarketData(market_data))
                                .await?;
                        }
                    }
                    "trade" => {
                        if let Some(trade_data) =
                            super::parser::parse_binance_trade(data, &self.symbol_mapper)?
                        {
                            self.data_sender.send(Message::Trade(trade_data)).await?;
                        }
                    }
                    _ => {
                        debug!("Unhandled event type: {}", event_type);
                    }
                }
            }
        } else if value.get("result").is_some() {
            // Response to subscription
            debug!("Subscription response: {:?}", value);
        } else if value.get("error").is_some() {
            let error_msg = format!("Binance error: {:?}", value["error"]);
            error!("{}", error_msg);
            self.data_sender.send(Message::Error(error_msg)).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl ExchangeConnection for BinanceConnection {
    async fn connect(&mut self) -> Result<()> {
        // For initial connection, we don't subscribe to streams yet
        let url = format!("{}/ws/dummy", self.base_url);
        info!("Connecting to Binance WebSocket: {}", url);

        let (ws_stream, _) = connect_async(&url).await?;
        *self.ws_stream.lock().await = Some(ws_stream);

        info!("Connected to Binance WebSocket");
        Ok(())
    }

    async fn subscribe(&mut self, channels: Vec<Channel>) -> Result<()> {
        // For Binance, we need to reconnect with the stream URL
        let stream_url = self.build_stream_url(&channels);
        info!("Connecting to Binance streams: {}", stream_url);

        // Close existing connection
        *self.ws_stream.lock().await = None;

        // Connect to combined stream
        let (ws_stream, _) = connect_async(&stream_url).await?;
        *self.ws_stream.lock().await = Some(ws_stream);

        info!(
            "Subscribed to {} symbols with {} channels",
            self.symbols.len(),
            channels.len()
        );
        Ok(())
    }

    async fn read_message(&mut self) -> Result<Option<Value>> {
        let mut stream = self.ws_stream.lock().await;
        if let Some(ws) = &mut *stream {
            match ws.next().await {
                Some(Ok(WsMessage::Text(text))) => {
                    drop(stream); // Release lock before processing
                    self.process_message(&text).await?;
                    Ok(Some(serde_json::from_str(&text)?))
                }
                Some(Ok(WsMessage::Ping(data))) => {
                    ws.send(WsMessage::Pong(data)).await?;
                    Ok(None)
                }
                Some(Ok(WsMessage::Pong(_))) => {
                    debug!("Received pong from Binance");
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
        let mut stream = self.ws_stream.lock().await;
        if let Some(ws) = &mut *stream {
            ws.send(WsMessage::Ping(vec![])).await?;
            debug!("Sent ping to Binance");
            Ok(())
        } else {
            Err(anyhow!("WebSocket not connected"))
        }
    }

    async fn reconnect(&mut self) -> Result<()> {
        *self.ws_stream.lock().await = None;
        self.connect().await?;
        self.subscribe(vec![Channel::Ticker, Channel::Trades]).await
    }

    fn is_connected(&self) -> bool {
        // This is a blocking operation, but should be very quick
        if let Ok(stream) = self.ws_stream.try_lock() {
            stream.is_some()
        } else {
            // If we can't get the lock, assume connected
            true
        }
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
                    members: vec!["USDT".to_string()],
                    primary: "USD".to_string(),
                }],
            },
        };

        let mapper = Arc::new(crate::common::SymbolMapper::new(config).unwrap());
        let (tx, _rx) = mpsc::channel(100);
        let conn = BinanceConnection::new(
            "wss://stream.binance.com:9443".to_string(),
            vec!["BTCUSDT".to_string()],
            tx,
            20,
            mapper,
        );

        assert_eq!(conn.symbols(), &["BTCUSDT"]);
        assert!(!conn.is_connected());
    }

    #[test]
    fn test_stream_url_building() {
        use crate::config::{AssetGroup, EquivalenceRules, SymbolMappingsConfig};

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

        let mapper = Arc::new(crate::common::SymbolMapper::new(config).unwrap());
        let (tx, _rx) = mpsc::channel(100);
        let conn = BinanceConnection::new(
            "wss://stream.binance.com:9443".to_string(),
            vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()],
            tx,
            20,
            mapper,
        );

        let url = conn.build_stream_url(&[Channel::Ticker, Channel::Trades]);
        assert_eq!(
            url,
            "wss://stream.binance.com:9443/stream?streams=btcusdt@ticker/btcusdt@trade/ethusdt@ticker/ethusdt@trade"
        );
    }
}
