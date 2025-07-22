//! WebSocket client for live data feeds
//!
//! This module implements WebSocket connectivity for real-time market data
//! with automatic reconnection, heartbeat, and subscription management.

use futures_util::{SinkExt, StreamExt};
use gpu_charts_shared::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

/// WebSocket configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// Reconnection delay
    pub reconnect_delay: Duration,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Message buffer size
    pub buffer_size: usize,
    /// Enable auto-reconnect
    pub auto_reconnect: bool,
    /// Request timeout
    pub request_timeout: Duration,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            reconnect_delay: Duration::from_secs(5),
            max_reconnect_attempts: 10,
            heartbeat_interval: Duration::from_secs(30),
            buffer_size: 1000,
            auto_reconnect: true,
            request_timeout: Duration::from_secs(10),
        }
    }
}

/// WebSocket connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

/// Market data subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub symbol: String,
    pub channels: Vec<String>,
    pub params: HashMap<String, serde_json::Value>,
}

/// Market data message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDataMessage {
    pub timestamp: u64,
    pub symbol: String,
    pub channel: String,
    pub data: serde_json::Value,
}

/// WebSocket client for live data feeds
pub struct WebSocketClient {
    config: WebSocketConfig,
    url: String,

    /// Current connection state
    state: Arc<RwLock<ConnectionState>>,

    /// Active subscriptions
    subscriptions: Arc<RwLock<HashMap<String, Subscription>>>,

    /// Message channels
    message_tx: mpsc::Sender<MarketDataMessage>,
    message_rx: Arc<RwLock<mpsc::Receiver<MarketDataMessage>>>,

    /// State broadcast
    state_tx: broadcast::Sender<ConnectionState>,

    /// Connection handle
    connection_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,

    /// Metrics
    metrics: Arc<RwLock<WebSocketMetrics>>,
}

/// WebSocket performance metrics
#[derive(Debug, Default)]
struct WebSocketMetrics {
    messages_received: u64,
    messages_processed: u64,
    bytes_received: u64,
    reconnection_count: u32,
    last_message_time: Option<Instant>,
    average_latency_ms: f32,
}

impl WebSocketClient {
    /// Create new WebSocket client
    pub fn new(url: String, config: WebSocketConfig) -> Self {
        let (message_tx, message_rx) = mpsc::channel(config.buffer_size);
        let (state_tx, _) = broadcast::channel(10);

        Self {
            config,
            url,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            message_tx,
            message_rx: Arc::new(RwLock::new(message_rx)),
            state_tx,
            connection_handle: Arc::new(RwLock::new(None)),
            metrics: Arc::new(RwLock::new(WebSocketMetrics::default())),
        }
    }

    /// Connect to WebSocket server
    pub async fn connect(&self) -> Result<()> {
        self.set_state(ConnectionState::Connecting).await;

        // Cancel existing connection if any
        if let Some(handle) = self.connection_handle.write().await.take() {
            handle.abort();
        }

        let url = self.url.clone();
        let state = Arc::clone(&self.state);
        let subscriptions = Arc::clone(&self.subscriptions);
        let message_tx = self.message_tx.clone();
        let metrics = Arc::clone(&self.metrics);
        let config = self.config.clone();
        let state_tx = self.state_tx.clone();

        // Spawn connection task
        let handle = tokio::spawn(async move {
            let mut reconnect_attempts = 0;

            loop {
                match Self::connect_internal(
                    &url,
                    &state,
                    &subscriptions,
                    &message_tx,
                    &metrics,
                    &config,
                )
                .await
                {
                    Ok(_) => {
                        reconnect_attempts = 0;
                        if !config.auto_reconnect {
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("WebSocket error: {}", e);

                        *state.write().await = ConnectionState::Error;
                        let _ = state_tx.send(ConnectionState::Error);

                        if !config.auto_reconnect
                            || reconnect_attempts >= config.max_reconnect_attempts
                        {
                            break;
                        }

                        reconnect_attempts += 1;
                        *state.write().await = ConnectionState::Reconnecting;
                        let _ = state_tx.send(ConnectionState::Reconnecting);

                        tokio::time::sleep(config.reconnect_delay).await;
                    }
                }
            }

            *state.write().await = ConnectionState::Disconnected;
            let _ = state_tx.send(ConnectionState::Disconnected);
        });

        *self.connection_handle.write().await = Some(handle);

        // Wait for connection
        let timeout = tokio::time::timeout(
            self.config.request_timeout,
            self.wait_for_state(ConnectionState::Connected),
        )
        .await;

        match timeout {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::Custom("Connection timeout".into())),
        }
    }

    /// Internal connection handler
    async fn connect_internal(
        url: &str,
        state: &Arc<RwLock<ConnectionState>>,
        subscriptions: &Arc<RwLock<HashMap<String, Subscription>>>,
        message_tx: &mpsc::Sender<MarketDataMessage>,
        metrics: &Arc<RwLock<WebSocketMetrics>>,
        config: &WebSocketConfig,
    ) -> Result<()> {
        // Connect to WebSocket
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| Error::Custom(format!("WebSocket connection failed: {}", e)))?;

        *state.write().await = ConnectionState::Connected;

        let (mut write, mut read) = ws_stream.split();

        // Re-subscribe to all active subscriptions
        let subs = subscriptions.read().await.clone();
        for sub in subs.values() {
            let msg = serde_json::json!({
                "type": "subscribe",
                "subscription": sub,
            });

            write
                .send(Message::Text(msg.to_string()))
                .await
                .map_err(|e| Error::Custom(format!("Failed to send subscription: {}", e)))?;
        }

        // Spawn heartbeat task
        let heartbeat_interval = config.heartbeat_interval;
        let (heartbeat_tx, mut heartbeat_rx) = mpsc::channel::<()>(1);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(heartbeat_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if heartbeat_tx.send(()).await.is_err() {
                            break;
                        }
                    }
                    _ = heartbeat_rx.recv() => {
                        break;
                    }
                }
            }
        });

        // Message handling loop
        let mut heartbeat_rx = heartbeat_tx.subscribe();

        loop {
            tokio::select! {
                Some(msg) = read.next() => {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Ok(data) = serde_json::from_str::<MarketDataMessage>(&text) {
                                // Update metrics
                                let mut metrics = metrics.write().await;
                                metrics.messages_received += 1;
                                metrics.bytes_received += text.len() as u64;
                                metrics.last_message_time = Some(Instant::now());

                                // Send to processor
                                let _ = message_tx.send(data).await;
                            }
                        }
                        Ok(Message::Binary(bin)) => {
                            // Handle binary messages
                            let mut metrics = metrics.write().await;
                            metrics.bytes_received += bin.len() as u64;
                        }
                        Ok(Message::Close(_)) => {
                            log::info!("WebSocket closed by server");
                            break;
                        }
                        Ok(Message::Ping(data)) => {
                            write.send(Message::Pong(data)).await
                                .map_err(|e| Error::Custom(format!("Failed to send pong: {}", e)))?;
                        }
                        Ok(Message::Pong(_)) => {
                            // Pong received
                        }
                        Ok(Message::Frame(_)) => {
                            // Raw frame
                        }
                        Err(e) => {
                            log::error!("WebSocket error: {}", e);
                            break;
                        }
                    }
                }
                _ = heartbeat_rx.recv() => {
                    // Send heartbeat
                    write.send(Message::Ping(vec![])).await
                        .map_err(|e| Error::Custom(format!("Failed to send heartbeat: {}", e)))?;
                }
            }
        }

        Ok(())
    }

    /// Subscribe to market data
    pub async fn subscribe(&self, subscription: Subscription) -> Result<()> {
        // Store subscription
        self.subscriptions
            .write()
            .await
            .insert(subscription.id.clone(), subscription.clone());

        // Send subscription if connected
        if self.get_state().await == ConnectionState::Connected {
            // Would send through WebSocket here
            log::info!("Subscribed to {}", subscription.id);
        }

        Ok(())
    }

    /// Unsubscribe from market data
    pub async fn unsubscribe(&self, subscription_id: &str) -> Result<()> {
        self.subscriptions.write().await.remove(subscription_id);

        // Send unsubscribe if connected
        if self.get_state().await == ConnectionState::Connected {
            // Would send through WebSocket here
            log::info!("Unsubscribed from {}", subscription_id);
        }

        Ok(())
    }

    /// Process incoming messages
    pub async fn process_messages<F, Fut>(&self, mut handler: F) -> Result<()>
    where
        F: FnMut(MarketDataMessage) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let mut rx = self.message_rx.write().await;

        while let Some(msg) = rx.recv().await {
            let start_time = Instant::now();

            handler(msg).await?;

            // Update metrics
            let mut metrics = self.metrics.write().await;
            metrics.messages_processed += 1;

            let latency_ms = start_time.elapsed().as_millis() as f32;
            metrics.average_latency_ms = (metrics.average_latency_ms * 0.9) + (latency_ms * 0.1);
        }

        Ok(())
    }

    /// Get current connection state
    pub async fn get_state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Set connection state
    async fn set_state(&self, state: ConnectionState) {
        *self.state.write().await = state;
        let _ = self.state_tx.send(state);
    }

    /// Wait for specific state
    async fn wait_for_state(&self, target_state: ConnectionState) {
        let mut rx = self.state_tx.subscribe();

        while let Ok(state) = rx.recv().await {
            if state == target_state {
                break;
            }
        }
    }

    /// Subscribe to state changes
    pub fn subscribe_state(&self) -> broadcast::Receiver<ConnectionState> {
        self.state_tx.subscribe()
    }

    /// Disconnect from WebSocket
    pub async fn disconnect(&self) {
        if let Some(handle) = self.connection_handle.write().await.take() {
            handle.abort();
        }

        self.set_state(ConnectionState::Disconnected).await;
    }

    /// Get WebSocket statistics
    pub async fn get_stats(&self) -> serde_json::Value {
        let metrics = self.metrics.read().await;
        let state = self.get_state().await;
        let subscription_count = self.subscriptions.read().await.len();

        serde_json::json!({
            "state": format!("{:?}", state),
            "messages_received": metrics.messages_received,
            "messages_processed": metrics.messages_processed,
            "bytes_received": metrics.bytes_received,
            "reconnection_count": metrics.reconnection_count,
            "average_latency_ms": metrics.average_latency_ms,
            "last_message_ago_ms": metrics.last_message_time
                .map(|t| t.elapsed().as_millis())
                .unwrap_or(0),
            "active_subscriptions": subscription_count,
        })
    }
}

/// Specialized WebSocket client for Coinbase
pub struct CoinbaseWebSocketClient {
    base_client: WebSocketClient,
}

impl CoinbaseWebSocketClient {
    pub fn new() -> Self {
        let config = WebSocketConfig::default();
        let client =
            WebSocketClient::new("wss://ws-feed.exchange.coinbase.com".to_string(), config);

        Self {
            base_client: client,
        }
    }

    /// Subscribe to ticker channel
    pub async fn subscribe_ticker(&self, symbols: Vec<String>) -> Result<()> {
        let subscription = Subscription {
            id: format!("ticker_{}", symbols.join("_")),
            symbol: symbols.join(","),
            channels: vec!["ticker".to_string()],
            params: HashMap::from([("product_ids".to_string(), serde_json::json!(symbols))]),
        };

        self.base_client.subscribe(subscription).await
    }

    /// Subscribe to level2 order book
    pub async fn subscribe_level2(&self, symbols: Vec<String>) -> Result<()> {
        let subscription = Subscription {
            id: format!("level2_{}", symbols.join("_")),
            symbol: symbols.join(","),
            channels: vec!["level2".to_string()],
            params: HashMap::from([("product_ids".to_string(), serde_json::json!(symbols))]),
        };

        self.base_client.subscribe(subscription).await
    }

    /// Subscribe to matches (trades)
    pub async fn subscribe_trades(&self, symbols: Vec<String>) -> Result<()> {
        let subscription = Subscription {
            id: format!("matches_{}", symbols.join("_")),
            symbol: symbols.join(","),
            channels: vec!["matches".to_string()],
            params: HashMap::from([("product_ids".to_string(), serde_json::json!(symbols))]),
        };

        self.base_client.subscribe(subscription).await
    }
}

/// Message parser for different exchange formats
pub trait MessageParser {
    fn parse(&self, raw: &str) -> Result<MarketDataMessage>;
}

/// Coinbase message parser
pub struct CoinbaseMessageParser;

impl MessageParser for CoinbaseMessageParser {
    fn parse(&self, raw: &str) -> Result<MarketDataMessage> {
        let value: serde_json::Value = serde_json::from_str(raw)
            .map_err(|e| Error::Custom(format!("Failed to parse message: {}", e)))?;

        let msg_type = value["type"].as_str().unwrap_or("");
        let product_id = value["product_id"].as_str().unwrap_or("");
        let timestamp = value["time"]
            .as_str()
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .map(|t| t.timestamp_millis() as u64)
            .unwrap_or(0);

        Ok(MarketDataMessage {
            timestamp,
            symbol: product_id.to_string(),
            channel: msg_type.to_string(),
            data: value,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_creation() {
        let sub = Subscription {
            id: "test".to_string(),
            symbol: "BTC-USD".to_string(),
            channels: vec!["ticker".to_string()],
            params: HashMap::new(),
        };

        assert_eq!(sub.id, "test");
        assert_eq!(sub.symbol, "BTC-USD");
        assert!(sub.channels.contains(&"ticker".to_string()));
    }

    #[test]
    fn test_coinbase_parser() {
        let parser = CoinbaseMessageParser;
        let raw = r#"{
            "type": "ticker",
            "product_id": "BTC-USD",
            "time": "2024-01-01T00:00:00.000Z",
            "price": "50000.00"
        }"#;

        let msg = parser.parse(raw).unwrap();
        assert_eq!(msg.symbol, "BTC-USD");
        assert_eq!(msg.channel, "ticker");
    }
}
