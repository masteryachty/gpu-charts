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

pub struct KrakenConnection {
    url: String,
    symbols: Vec<String>,
    data_sender: mpsc::Sender<Message>,
    ws_stream: Option<WsStream>,
    subscription_id: u64,
}

impl KrakenConnection {
    pub fn new(url: String, symbols: Vec<String>, data_sender: mpsc::Sender<Message>) -> Self {
        Self {
            url,
            symbols,
            data_sender,
            ws_stream: None,
            subscription_id: 0,
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

        // Kraken sends data as arrays for channel data
        if let Some(arr) = value.as_array() {
            if arr.len() >= 4 {
                // Format: [channelID, data, channelName, pair]
                let channel_name = arr[2].as_str().unwrap_or("");
                let pair = arr[3].as_str().unwrap_or("");

                match channel_name {
                    "ticker" => {
                        debug!("Processing Kraken ticker for {}", pair);
                        if let Some(data) = super::parser::parse_kraken_ticker_array(&arr[1], pair)?
                        {
                            self.data_sender.send(Message::MarketData(data)).await?;
                        }
                    }
                    "trade" => {
                        debug!("Processing Kraken trades for {}", pair);
                        if let Some(trades) = arr[1].as_array() {
                            for trade in trades {
                                if let Some(data) =
                                    super::parser::parse_kraken_trade_array(trade, pair)?
                                {
                                    self.data_sender.send(Message::Trade(data)).await?;
                                }
                            }
                        }
                    }
                    _ => {
                        debug!(
                            "Unhandled Kraken channel: {} for pair: {}",
                            channel_name, pair
                        );
                    }
                }
            } else if arr.len() > 0 {
                debug!(
                    "Kraken array message with {} elements: {:?}",
                    arr.len(),
                    arr
                );
            }
        } else if let Some(obj) = value.as_object() {
            // Handle system messages
            if let Some(event) = obj.get("event").and_then(|v| v.as_str()) {
                match event {
                    "heartbeat" => {
                        self.data_sender.send(Message::Heartbeat).await?;
                    }
                    "pong" => {
                        debug!("Received pong");
                    }
                    "subscriptionStatus" => {
                        let status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
                        let channel = obj
                            .get("channelName")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let pair = obj.get("pair").and_then(|v| v.as_str()).unwrap_or("");

                        if status == "subscribed" {
                            debug!(
                                "Successfully subscribed to Kraken channel: {} for pair: {}",
                                channel, pair
                            );
                        } else if status == "error" {
                            let error_msg = obj
                                .get("errorMessage")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown error");
                            error!("Kraken subscription error for {}: {}", pair, error_msg);
                            self.data_sender
                                .send(Message::Error(error_msg.to_string()))
                                .await?;
                        }
                    }
                    "systemStatus" => {
                        let status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
                        info!("Kraken system status: {}", status);
                    }
                    "error" => {
                        let error_msg = obj
                            .get("errorMessage")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown error");
                        error!("Kraken error: {}", error_msg);
                        self.data_sender
                            .send(Message::Error(error_msg.to_string()))
                            .await?;
                    }
                    _ => {
                        debug!("Unhandled event: {}", event);
                    }
                }
            }
        }

        Ok(())
    }

    fn generate_subscription_id(&mut self) -> u64 {
        self.subscription_id += 1;
        self.subscription_id
    }
}

#[async_trait]
impl ExchangeConnection for KrakenConnection {
    async fn connect(&mut self) -> Result<()> {
        info!("Connecting to Kraken WebSocket: {}", self.url);

        let (ws_stream, _) = connect_async(&self.url).await?;
        self.ws_stream = Some(ws_stream);

        info!("Connected to Kraken WebSocket");
        Ok(())
    }

    async fn subscribe(&mut self, channels: Vec<Channel>) -> Result<()> {
        let num_channels = channels.len();
        for channel in channels {
            let channel_name = match channel {
                Channel::Ticker => "ticker",
                Channel::Trades => "trade",
                Channel::OrderBook => "book",
            };

            let subscribe_msg = json!({
                "event": "subscribe",
                "pair": &self.symbols,
                "subscription": {
                    "name": channel_name
                },
                "reqid": self.generate_subscription_id()
            });

            debug!("Sending Kraken subscription: {}", subscribe_msg.to_string());
            self.send_json(subscribe_msg).await?;
        }

        info!(
            "Sent subscription requests for {} symbols with {} channels",
            self.symbols.len(),
            num_channels
        );
        Ok(())
    }

    async fn read_message(&mut self) -> Result<Option<Value>> {
        if let Some(ws) = &mut self.ws_stream {
            match ws.next().await {
                Some(Ok(WsMessage::Text(text))) => {
                    if let Err(e) = self.process_message(&text).await {
                        // Log error but don't fail the connection for parse errors
                        debug!("Failed to process message: {}", e);
                    }
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
        // Kraken uses JSON ping messages
        let ping_msg = json!({
            "event": "ping",
            "reqid": self.generate_subscription_id()
        });

        self.send_json(ping_msg).await?;
        debug!("Sent ping to Kraken");
        Ok(())
    }

    async fn reconnect(&mut self) -> Result<()> {
        self.ws_stream = None;
        self.subscription_id = 0;
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
        let conn = KrakenConnection::new(
            "wss://ws.kraken.com".to_string(),
            vec!["XBT/USD".to_string()],
            tx,
        );

        assert_eq!(conn.symbols(), &["XBT/USD"]);
        assert!(!conn.is_connected());
    }
}
