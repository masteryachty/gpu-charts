use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{protocol::WebSocketConfig, Message},
};

use crate::Result;

pub async fn get_all_products() -> Result<Vec<String>> {
    // Try REST API first as fallback
    match get_products_from_rest_api().await {
        Ok(products) => {
            println!("Found {} products from REST API", products.len());
            return Ok(products);
        }
        Err(e) => {
            println!("REST API failed: {e}, trying WebSocket...");
        }
    }

    get_products_from_websocket().await
}

async fn get_products_from_rest_api() -> Result<Vec<String>> {
    println!("Calling REST API...");
    
    println!("Creating reqwest client...");
    let client = reqwest::Client::new();
    
    println!("Making GET request to https://api.exchange.coinbase.com/products");
    let response = match client.get("https://api.exchange.coinbase.com/products").send().await {
        Ok(resp) => {
            println!("HTTP request successful, status: {}", resp.status());
            resp
        }
        Err(e) => {
            println!("HTTP request failed: {:?}", e);
            return Err(format!("HTTP request failed: {}", e).into());
        }
    };
    
    println!("Parsing JSON response...");
    let json_result = match response.json::<serde_json::Value>().await {
        Ok(json) => {
            println!("JSON parsing successful");
            json
        }
        Err(e) => {
            println!("JSON parsing failed: {:?}", e);
            return Err(format!("JSON parsing failed: {}", e).into());
        }
    };

    println!("REST API response received");
    if let Some(products_array) = json_result.as_array() {
        println!(
            "Found {} products in REST API response",
            products_array.len()
        );
        let products = products_array
            .iter()
            .filter_map(|p| {
                if p.get("status").and_then(|s| s.as_str()) == Some("online") {
                    p.get("id").and_then(|id| id.as_str()).map(String::from)
                } else {
                    None
                }
            })
            .collect::<Vec<String>>();
        println!("Found {} online products", products.len());
        Ok(products)
    } else {
        Err("Invalid response format from REST API".into())
    }
}

async fn get_products_from_websocket() -> Result<Vec<String>> {
    println!("Fetching all available products from Coinbase...");

    let (ws_stream, _) = connect_async_with_config(
        "wss://ws-feed.exchange.coinbase.com",
        Some(WebSocketConfig::default()),
        true,
    )
    .await?;

    println!("WebSocket connection established");

    let (mut write, mut read) = ws_stream.split();

    // Subscribe to status channel
    let subscribe_msg = json!({
        "type": "subscribe",
        "channels": [{
            "name": "status"
        }]
    });
    println!("Sending subscription message: {subscribe_msg}");
    write.send(Message::Text(subscribe_msg.to_string())).await?;

    // Wait for status message with timeout
    let mut message_count = 0;
    while let Some(message) = read.next().await {
        message_count += 1;
        println!("Received message #{message_count}: {message:?}");

        if let Ok(msg) = message {
            if msg.is_text() {
                let text = msg.into_text()?;
                println!("Message text: {text}");

                let v: serde_json::Value = serde_json::from_str(&text)?;
                println!("Parsed JSON: {v}");

                if v.get("type") == Some(&json!("status")) {
                    println!("Found status message!");
                    if let Some(products_array) = v.get("products").and_then(|p| p.as_array()) {
                        println!("Found products array with {} items", products_array.len());
                        let products: Vec<String> = products_array
                            .iter()
                            .filter_map(|p| {
                                if p.get("status").and_then(|s| s.as_str()) == Some("online") {
                                    p.get("id").and_then(|id| id.as_str()).map(String::from)
                                } else {
                                    None
                                }
                            })
                            .collect();
                        println!("Found {} online products", products.len());
                        return Ok(products);
                    }
                }
            }
        }

        // Safety timeout - don't wait forever
        if message_count > 10 {
            println!("Received {message_count} messages but no status message, giving up");
            break;
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
