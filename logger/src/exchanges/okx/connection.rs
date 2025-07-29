use crate::common::SymbolMapper;
use crate::exchanges::{Channel, ExchangeConnection, Message};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message as WsMessage, MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info, warn};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct OkxConnection {
    url: String,
    symbols: Vec<String>,
    data_sender: mpsc::Sender<Message>,
    ws_stream: Option<Arc<Mutex<WsStream>>>,
    symbol_mapper: Arc<SymbolMapper>,
}

impl OkxConnection {
    pub fn new(
        url: String,
        symbols: Vec<String>,
        data_sender: mpsc::Sender<Message>,
        symbol_mapper: Arc<SymbolMapper>,
    ) -> Self {
        Self {
            url,
            symbols,
            data_sender,
            ws_stream: None,
            symbol_mapper,
        }
    }

    pub fn clone_for_ping(&self) -> Self {
        Self {
            url: self.url.clone(),
            symbols: self.symbols.clone(),
            data_sender: self.data_sender.clone(),
            ws_stream: self.ws_stream.clone(),
            symbol_mapper: self.symbol_mapper.clone(),
        }
    }

    async fn send_json(&mut self, payload: Value) -> Result<()> {
        if let Some(ws_arc) = &self.ws_stream {
            let mut ws = ws_arc.lock().await;
            let msg = WsMessage::Text(payload.to_string());
            ws.send(msg).await?;
            Ok(())
        } else {
            Err(anyhow!("WebSocket not connected"))
        }
    }

    async fn process_message(&self, text: &str) -> Result<()> {
        // OKX might send "pong" as a plain text response to ping
        if text == "pong" {
            debug!("Received pong from OKX");
            return Ok(());
        }

        let value: Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(_) => {
                // Not JSON, might be a plain text message
                debug!("Received non-JSON message: {}", text);
                return Ok(());
            }
        };

        // OKX sends messages in two formats:
        // 1. Response to subscription: {"event":"subscribe","arg":{"channel":"tickers","instId":"BTC-USDT"}}
        // 2. Data messages: {"arg":{"channel":"tickers","instId":"BTC-USDT"},"data":[...]}

        if let Some(event) = value["event"].as_str() {
            match event {
                "subscribe" => {
                    info!("Subscribed to channel: {}", value["arg"]);
                }
                "error" => {
                    let error_msg = format!(
                        "OKX error: {} - {}",
                        value["code"].as_str().unwrap_or(""),
                        value["msg"].as_str().unwrap_or("Unknown error")
                    );
                    error!("{}", error_msg);
                    self.data_sender.send(Message::Error(error_msg)).await?;
                }
                _ => {
                    debug!("Unhandled event type: {}", event);
                }
            }
            return Ok(());
        }

        // Handle data messages
        if let Some(arg) = value["arg"].as_object() {
            if let (Some(channel), Some(data_array)) =
                (arg["channel"].as_str(), value["data"].as_array())
            {
                match channel {
                    "tickers" => {
                        for data in data_array {
                            if let Some(market_data) =
                                super::parser::parse_okx_ticker(data, &self.symbol_mapper)?
                            {
                                self.data_sender
                                    .send(Message::MarketData(market_data))
                                    .await?;
                            }
                        }
                    }
                    "trades" => {
                        for data in data_array {
                            if let Some(trade_data) =
                                super::parser::parse_okx_trade(data, &self.symbol_mapper)?
                            {
                                self.data_sender.send(Message::Trade(trade_data)).await?;
                            }
                        }
                    }
                    _ => {
                        debug!("Unhandled channel: {}", channel);
                    }
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ExchangeConnection for OkxConnection {
    async fn connect(&mut self) -> Result<()> {
        info!("Connecting to OKX WebSocket: {}", self.url);

        let (ws_stream, _) = connect_async(&self.url).await?;
        self.ws_stream = Some(Arc::new(Mutex::new(ws_stream)));

        info!("Connected to OKX WebSocket");
        Ok(())
    }

    async fn subscribe(&mut self, channels: Vec<Channel>) -> Result<()> {
        let mut args = Vec::new();

        for channel in channels {
            let channel_name = match channel {
                Channel::Ticker => "tickers",
                Channel::Trades => "trades",
                Channel::OrderBook => "books5",
            };

            // OKX requires subscribing to each symbol individually
            for symbol in &self.symbols {
                args.push(json!({
                    "channel": channel_name,
                    "instId": symbol
                }));
            }
        }

        // OKX supports batch subscription
        let subscribe_msg = json!({
            "op": "subscribe",
            "args": args
        });

        self.send_json(subscribe_msg).await?;
        info!("Subscribed to {} symbols on OKX", self.symbols.len());

        Ok(())
    }

    async fn read_message(&mut self) -> Result<Option<Value>> {
        if let Some(ws_arc) = &self.ws_stream {
            let mut ws = ws_arc.lock().await;

            match ws.next().await {
                Some(Ok(WsMessage::Text(text))) => {
                    // Process message in a separate task to avoid holding the lock
                    let text_clone = text.clone();
                    let self_clone = self.clone_for_ping();
                    tokio::spawn(async move {
                        if let Err(e) = self_clone.process_message(&text_clone).await {
                            error!("Error processing message: {}", e);
                        }
                    });

                    Ok(Some(serde_json::from_str(&text)?))
                }
                Some(Ok(WsMessage::Ping(data))) => {
                    ws.send(WsMessage::Pong(data)).await?;
                    Ok(None)
                }
                Some(Ok(WsMessage::Pong(_))) => {
                    debug!("Received pong from OKX");
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
        // OKX expects "ping" as a plain text message, not JSON
        if let Some(ws_arc) = &self.ws_stream {
            let mut ws = ws_arc.lock().await;
            ws.send(WsMessage::Text("ping".to_string())).await?;
            debug!("Sent ping to OKX");
            Ok(())
        } else {
            Err(anyhow!("WebSocket not connected"))
        }
    }

    async fn reconnect(&mut self) -> Result<()> {
        self.ws_stream = None;
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
                    members: vec!["USDT".to_string()],
                    primary: "USDT".to_string(),
                }],
            },
        };

        let mapper = Arc::new(crate::common::SymbolMapper::new(config).unwrap());
        let (tx, _rx) = mpsc::channel(100);
        let conn = OkxConnection::new(
            "wss://ws.okx.com:8443/ws/v5/public".to_string(),
            vec!["BTC-USDT".to_string()],
            tx,
            mapper,
        );

        assert_eq!(conn.symbols(), &["BTC-USDT"]);
        assert!(!conn.is_connected());
    }
}
