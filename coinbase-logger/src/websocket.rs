use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{protocol::WebSocketConfig, Message},
};

use crate::Result;

pub async fn get_all_products() -> Result<Vec<String>> {
    println!("Fetching all available products from Coinbase...");

    let (ws_stream, _) = connect_async_with_config(
        "wss://ws-feed.exchange.coinbase.com",
        Some(WebSocketConfig::default()),
        true,
    )
    .await?;

    let (mut write, mut read) = ws_stream.split();

    // Subscribe to status channel
    let subscribe_msg = json!({
        "type": "subscribe",
        "channels": [{
            "name": "status"
        }]
    });
    write.send(Message::Text(subscribe_msg.to_string())).await?;

    // Wait for status message
    while let Some(message) = read.next().await {
        if let Ok(msg) = message {
            if msg.is_text() {
                let text = msg.into_text()?;
                let v: serde_json::Value = serde_json::from_str(&text)?;

                if v.get("type") == Some(&json!("status")) {
                    if let Some(products_array) = v.get("products").and_then(|p| p.as_array()) {
                        let products = products_array
                            .iter()
                            .filter_map(|p| {
                                if p.get("status").and_then(|s| s.as_str()) == Some("online") {
                                    p.get("id").and_then(|id| id.as_str()).map(String::from)
                                } else {
                                    None
                                }
                            })
                            .collect();
                        return Ok(products);
                    }
                }
            }
        }
    }

    Err("No status message received from Coinbase".into())
}

pub fn create_websocket_config() -> WebSocketConfig {
    WebSocketConfig {
        max_message_size: Some(64 << 20),
        max_frame_size: Some(16 << 20),
        write_buffer_size: 256 * 1024,
        max_write_buffer_size: 512 * 1024,
        accept_unmasked_frames: false,
        ..Default::default()
    }
}
