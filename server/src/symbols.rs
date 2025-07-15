use serde_json::json;
use std::convert::Infallible;
use tokio::fs;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

use hyper::{body::Body, header, Response, StatusCode};

/// Handler for the /api/symbols endpoint.
pub async fn handle_symbols_request() -> Result<Response<Body>, Infallible> {
    // First try runtime env var, then fall back to compile-time config
    let base_path =
        std::env::var("DATA_PATH").unwrap_or_else(|_| env!("GRAPH_DATA_PATH").to_string());
    let mut symbols = Vec::new();

    match fs::read_dir(base_path).await {
        Ok(read_dir) => {
            let mut stream = ReadDirStream::new(read_dir);

            while let Some(entry_result) = stream.next().await {
                if let Ok(entry) = entry_result {
                    if let Ok(metadata) = entry.metadata().await {
                        if metadata.is_dir() {
                            if let Some(name) = entry.file_name().to_str() {
                                symbols.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to read data directory: {}", err);
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to read symbol directory"))
                .unwrap());
        }
    }

    let json = json!({ "symbols": symbols });
    let body = Body::from(json.to_string());

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(body)
        .unwrap())
}
