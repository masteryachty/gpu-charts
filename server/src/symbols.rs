use serde_json::json;
use std::convert::Infallible;
use tokio::fs;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

use hyper::{body::Body, header, Response, StatusCode};

/// Handler for the /api/symbols endpoint.
pub async fn handle_symbols_request() -> Result<Response<Body>, Infallible> {
    // Always use /mnt/md/data as the base path
    let base_path = "/mnt/md/data".to_string();
    let mut symbols = Vec::new();
    let mut exchanges = std::collections::HashMap::new();

    match fs::read_dir(&base_path).await {
        Ok(read_dir) => {
            let mut stream = ReadDirStream::new(read_dir);

            // First level: exchanges
            while let Some(entry_result) = stream.next().await {
                if let Ok(entry) = entry_result {
                    if let Ok(metadata) = entry.metadata().await {
                        if metadata.is_dir() {
                            if let Some(exchange_name) = entry.file_name().to_str() {
                                let mut exchange_symbols = Vec::new();
                                let exchange_path = format!("{base_path}/{exchange_name}");

                                // Second level: symbols
                                if let Ok(symbol_dir) = fs::read_dir(&exchange_path).await {
                                    let mut symbol_stream = ReadDirStream::new(symbol_dir);

                                    while let Some(symbol_entry_result) = symbol_stream.next().await
                                    {
                                        if let Ok(symbol_entry) = symbol_entry_result {
                                            if let Ok(symbol_metadata) =
                                                symbol_entry.metadata().await
                                            {
                                                if symbol_metadata.is_dir() {
                                                    if let Some(symbol_name) =
                                                        symbol_entry.file_name().to_str()
                                                    {
                                                        exchange_symbols
                                                            .push(symbol_name.to_string());
                                                        symbols.push(symbol_name.to_string());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                exchanges.insert(exchange_name.to_string(), exchange_symbols);
                            }
                        }
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to read data directory: {err}");
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to read symbol directory"))
                .unwrap());
        }
    }

    // Remove duplicates from symbols
    symbols.sort();
    symbols.dedup();

    let json = json!({
        "symbols": symbols,
        "exchanges": exchanges
    });
    let body = Body::from(json.to_string());

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(body)
        .unwrap())
}
