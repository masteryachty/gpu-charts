use crate::exchanges::{Channel, ExchangeConnection, Message};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message as WsMessage, MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info, warn};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct CoinbaseConnection {
    url: String,
    symbols: Vec<String>,
    data_sender: mpsc::Sender<Message>,
    ws_stream: Option<WsStream>,
}

impl CoinbaseConnection {
    pub fn new(url: String, symbols: Vec<String>, data_sender: mpsc::Sender<Message>) -> Self {
        Self {
            url,
            symbols,
            data_sender,
            ws_stream: None,
        }
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

        match value["type"].as_str() {
            Some("ticker") => {
                if let Some(data) = super::parser::parse_coinbase_ticker(&value)? {
                    self.data_sender.send(Message::MarketData(data)).await?;
                }
            }
            Some("match") => {
                if let Some(data) = super::parser::parse_coinbase_trade(&value)? {
                    self.data_sender.send(Message::Trade(data)).await?;
                }
            }
            Some("subscriptions") => {
                debug!("Subscribed to channels: {}", value["channels"]);
            }
            Some("error") => {
                let error_msg = value["message"].as_str().unwrap_or("Unknown error");
                error!("Coinbase error: {}", error_msg);
                self.data_sender
                    .send(Message::Error(error_msg.to_string()))
                    .await?;
            }
            Some("heartbeat") => {
                self.data_sender.send(Message::Heartbeat).await?;
            }
            _ => {
                debug!("Unhandled message type: {:?}", value["type"]);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ExchangeConnection for CoinbaseConnection {
    async fn connect(&mut self) -> Result<()> {
        info!("Connecting to Coinbase WebSocket: {}", self.url);

        let (ws_stream, _) = connect_async(&self.url).await?;
        self.ws_stream = Some(ws_stream);

        info!("Connected to Coinbase WebSocket");
        Ok(())
    }

    async fn subscribe(&mut self, channels: Vec<Channel>) -> Result<()> {
        let channel_names: Vec<&str> = channels
            .iter()
            .map(|c| match c {
                Channel::Ticker => "ticker",
                Channel::Trades => "matches",
                Channel::OrderBook => "level2",
            })
            .collect();

        // Always include heartbeat channel to keep connection alive
        let mut all_channels = channel_names;
        all_channels.push("heartbeat");

        let subscribe_msg = json!({
            "type": "subscribe",
            "product_ids": &self.symbols,
            "channels": all_channels
        });

        self.send_json(subscribe_msg).await?;
        info!("Subscribed to {} symbols", self.symbols.len());

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
        // Coinbase uses subscription-based heartbeats, not ping/pong
        // The heartbeat channel subscription keeps the connection alive
        Ok(())
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
        let (tx, _rx) = mpsc::channel(100);
        let conn = CoinbaseConnection::new(
            "wss://ws-feed.exchange.coinbase.com".to_string(),
            vec!["BTC-USD".to_string()],
            tx,
        );

        assert_eq!(conn.symbols(), &["BTC-USD"]);
        assert!(!conn.is_connected());
    }
}
