mod search;

pub use search::{
    SymbolSearchService, 
    SearchResult, 
    ExchangeSymbol,
    initialize_search_service,
    SEARCH_SERVICE
};

// Re-export existing symbols functionality
use serde_json::json;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

use hyper::{body::Body, header, Response, StatusCode};

// Global symbol registry loaded at startup
pub static SYMBOL_REGISTRY: once_cell::sync::Lazy<Arc<RwLock<SymbolRegistry>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(SymbolRegistry::default())));

#[derive(Debug, Clone, Default)]
pub struct SymbolRegistry {
    pub exchanges: HashMap<String, Vec<String>>,
    pub all_symbols: Vec<String>,
    pub json_cache: String,  // Pre-serialized JSON for instant responses
}

/// Load all symbols at startup - called once from main.rs
pub async fn load_symbols_at_startup() -> Result<(), Box<dyn std::error::Error>> {
    println!("Loading symbol registry...");
    let start = std::time::Instant::now();
    
    let base_path = "/mnt/md/data";
    let mut exchanges: HashMap<String, Vec<String>> = HashMap::new();
    
    // Read exchange directories
    let mut entries = fs::read_dir(base_path).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        if let Some(exchange_name) = entry.file_name().to_str() {
            // Skip hidden files and non-directories
            if !exchange_name.starts_with('.') && !exchange_name.contains('.') {
                let exchange_path = format!("{}/{}", base_path, exchange_name);
                let mut symbols = Vec::new();
                
                // Read symbol directories for this exchange
                let mut symbol_entries = fs::read_dir(&exchange_path).await?;
                
                while let Some(symbol_entry) = symbol_entries.next_entry().await? {
                    if let Some(symbol_name) = symbol_entry.file_name().to_str() {
                        // Skip hidden files and non-directories
                        if !symbol_name.starts_with('.') && !symbol_name.contains('.') {
                            symbols.push(symbol_name.to_string());
                        }
                    }
                }
                
                // Sort symbols alphabetically
                symbols.sort_unstable();
                
                if !symbols.is_empty() {
                    println!("  Exchange '{}': {} symbols", exchange_name, symbols.len());
                    exchanges.insert(exchange_name.to_string(), symbols);
                }
            }
        }
    }
    
    // Build all_symbols list
    let mut all_symbols = Vec::new();
    for symbols in exchanges.values() {
        for symbol in symbols {
            all_symbols.push(symbol.clone());
        }
    }
    
    // Remove duplicates
    all_symbols.sort_unstable();
    all_symbols.dedup();
    
    // Pre-serialize the JSON response
    let response_data = json!({
        "symbols": all_symbols.clone(),
        "exchanges": exchanges.clone()
    });
    let json_cache = response_data.to_string();
    
    // Store in global registry
    let mut registry = SYMBOL_REGISTRY.write().await;
    registry.exchanges = exchanges;
    registry.all_symbols = all_symbols;
    registry.json_cache = json_cache;
    
    let elapsed = start.elapsed();
    println!("Symbol registry loaded in {:.2}ms", elapsed.as_millis());
    println!("  Total symbols: {}", registry.all_symbols.len());
    println!("  Total exchanges: {}", registry.exchanges.len());
    
    Ok(())
}

/// Handler for the /api/symbols endpoint - serves from memory
pub async fn handle_symbols_request(
    req: hyper::Request<Body>,
) -> Result<Response<Body>, Infallible> {
    // Parse query parameters for exchange filter
    let query = req.uri().query().unwrap_or("");
    let mut exchange_filter: Option<String> = None;
    
    if !query.is_empty() {
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                if key == "exchange" {
                    exchange_filter = Some(value.to_string());
                    break;
                }
            }
        }
    }
    
    // Get registry
    let registry = SYMBOL_REGISTRY.read().await;
    
    // If no filter, return pre-cached JSON instantly
    if exchange_filter.is_none() {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .header("X-Cache-Status", "MEMORY")
            .header("X-Fetch-Time-Ms", "0")
            .body(Body::from(registry.json_cache.clone()))
            .unwrap());
    }
    
    // Apply filter if needed
    let filtered_response = if let Some(filter) = exchange_filter {
        if let Some(symbols) = registry.exchanges.get(&filter) {
            json!({
                "symbols": symbols,
                "exchanges": {
                    filter: symbols
                }
            })
        } else {
            json!({
                "symbols": [],
                "exchanges": {}
            })
        }
    } else {
        // This shouldn't happen, but handle it anyway
        json!({
            "symbols": &registry.all_symbols,
            "exchanges": &registry.exchanges
        })
    };
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .header("X-Cache-Status", "MEMORY")
        .header("X-Fetch-Time-Ms", "0")
        .body(Body::from(filtered_response.to_string()))
        .unwrap())
}

/// Handler for the /api/symbol-search endpoint
pub async fn handle_symbol_search_request(
    req: hyper::Request<Body>,
) -> Result<Response<Body>, Infallible> {
    // Parse query parameters
    let query = req.uri().query().unwrap_or("");
    let mut search_query = "";
    
    if !query.is_empty() {
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                if key == "q" {
                    search_query = value;
                    break;
                }
            }
        }
    }
    
    // If no query provided, return empty results
    if search_query.is_empty() {
        let response = json!({
            "results": []
        });
        
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .body(Body::from(response.to_string()))
            .unwrap());
    }
    
    // URL decode the query
    let decoded_query = urlencoding::decode(search_query)
        .unwrap_or_else(|_| std::borrow::Cow::Borrowed(search_query))
        .to_string();
    
    // Perform search
    let service = SEARCH_SERVICE.read().await;
    let results = service.search(&decoded_query);
    
    let response = json!({
        "results": results
    });
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from(response.to_string()))
        .unwrap())
}