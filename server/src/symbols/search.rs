use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Symbol metadata for normalized representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMetadata {
    pub normalized_id: String,     // e.g., "BTC/USD"
    pub base: String,              // e.g., "BTC"
    pub quote: String,             // e.g., "USD"
    pub display_name: String,      // e.g., "Bitcoin / US Dollar"
    pub description: String,       // e.g., "Bitcoin to US Dollar spot trading pair"
    pub tags: Vec<String>,         // e.g., ["crypto", "major", "btc", "bitcoin", "usd"]
    pub category: String,          // e.g., "crypto", "forex", "commodity"
}

// Exchange-specific symbol mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeSymbol {
    pub exchange: String,          // e.g., "coinbase"
    pub symbol: String,            // e.g., "BTC-USD"
}

// Search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub normalized_id: String,
    pub display_name: String,
    pub description: String,
    pub base: String,
    pub quote: String,
    pub category: String,
    pub exchanges: Vec<ExchangeSymbol>,
    pub relevance_score: f32,
}

// Configuration for an exchange's symbol mappings
#[derive(Debug, Clone, Deserialize)]
pub struct ExchangeConfig {
    pub exchange_name: String,
    pub mappings: HashMap<String, SymbolMetadata>,
}

// Main search service
pub struct SymbolSearchService {
    // Normalized ID -> metadata
    normalized_symbols: HashMap<String, SymbolMetadata>,
    // Exchange -> Symbol -> Normalized ID
    exchange_mappings: HashMap<String, HashMap<String, String>>,
    // Search indices for fast lookup
    search_indices: SearchIndices,
}

struct SearchIndices {
    // Lowercase symbol parts for case-insensitive search
    by_base: HashMap<String, Vec<String>>,      // "btc" -> ["BTC/USD", "BTC/EUR"]
    by_quote: HashMap<String, Vec<String>>,     // "usd" -> ["BTC/USD", "ETH/USD"]
    by_tag: HashMap<String, Vec<String>>,       // "bitcoin" -> ["BTC/USD"]
    // Full text tokens from display names and descriptions
    text_tokens: HashMap<String, Vec<String>>,   // "bitcoin" -> ["BTC/USD"]
}

impl SymbolSearchService {
    pub fn new() -> Self {
        Self {
            normalized_symbols: HashMap::new(),
            exchange_mappings: HashMap::new(),
            search_indices: SearchIndices {
                by_base: HashMap::new(),
                by_quote: HashMap::new(),
                by_tag: HashMap::new(),
                text_tokens: HashMap::new(),
            },
        }
    }

    // Load configuration from embedded data
    pub fn load_configs(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Load each exchange configuration
        self.load_coinbase_config()?;
        self.load_binance_config()?;
        self.load_bitfinex_config()?;
        self.load_okx_config()?;
        self.load_kraken_config()?;
        
        // Build search indices
        self.build_indices();
        
        Ok(())
    }

    fn load_coinbase_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let config = include_str!("configs/coinbase.json");
        let mappings: HashMap<String, SymbolMetadata> = serde_json::from_str(config)?;
        
        let mut exchange_map = HashMap::new();
        for (symbol, metadata) in mappings {
            exchange_map.insert(symbol.clone(), metadata.normalized_id.clone());
            self.normalized_symbols.entry(metadata.normalized_id.clone())
                .or_insert(metadata);
        }
        
        self.exchange_mappings.insert("coinbase".to_string(), exchange_map);
        Ok(())
    }

    fn load_binance_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let config = include_str!("configs/binance.json");
        let mappings: HashMap<String, SymbolMetadata> = serde_json::from_str(config)?;
        
        let mut exchange_map = HashMap::new();
        for (symbol, metadata) in mappings {
            exchange_map.insert(symbol.clone(), metadata.normalized_id.clone());
            self.normalized_symbols.entry(metadata.normalized_id.clone())
                .or_insert(metadata);
        }
        
        self.exchange_mappings.insert("binance".to_string(), exchange_map);
        Ok(())
    }

    fn load_bitfinex_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let config = include_str!("configs/bitfinex.json");
        let mappings: HashMap<String, SymbolMetadata> = serde_json::from_str(config)?;
        
        let mut exchange_map = HashMap::new();
        for (symbol, metadata) in mappings {
            exchange_map.insert(symbol.clone(), metadata.normalized_id.clone());
            self.normalized_symbols.entry(metadata.normalized_id.clone())
                .or_insert(metadata);
        }
        
        self.exchange_mappings.insert("bitfinex".to_string(), exchange_map);
        Ok(())
    }

    fn load_okx_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let config = include_str!("configs/okx.json");
        let mappings: HashMap<String, SymbolMetadata> = serde_json::from_str(config)?;
        
        let mut exchange_map = HashMap::new();
        for (symbol, metadata) in mappings {
            exchange_map.insert(symbol.clone(), metadata.normalized_id.clone());
            self.normalized_symbols.entry(metadata.normalized_id.clone())
                .or_insert(metadata);
        }
        
        self.exchange_mappings.insert("okx".to_string(), exchange_map);
        Ok(())
    }

    fn load_kraken_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let config = include_str!("configs/kraken.json");
        let mappings: HashMap<String, SymbolMetadata> = serde_json::from_str(config)?;
        
        let mut exchange_map = HashMap::new();
        for (symbol, metadata) in mappings {
            exchange_map.insert(symbol.clone(), metadata.normalized_id.clone());
            self.normalized_symbols.entry(metadata.normalized_id.clone())
                .or_insert(metadata);
        }
        
        self.exchange_mappings.insert("kraken".to_string(), exchange_map);
        Ok(())
    }

    fn build_indices(&mut self) {
        for (normalized_id, metadata) in &self.normalized_symbols {
            // Index by base currency
            let base_lower = metadata.base.to_lowercase();
            self.search_indices.by_base
                .entry(base_lower)
                .or_insert_with(Vec::new)
                .push(normalized_id.clone());
            
            // Index by quote currency
            let quote_lower = metadata.quote.to_lowercase();
            self.search_indices.by_quote
                .entry(quote_lower)
                .or_insert_with(Vec::new)
                .push(normalized_id.clone());
            
            // Index by tags
            for tag in &metadata.tags {
                let tag_lower = tag.to_lowercase();
                self.search_indices.by_tag
                    .entry(tag_lower)
                    .or_insert_with(Vec::new)
                    .push(normalized_id.clone());
            }
            
            // Index text tokens from display name and description
            let text = format!("{} {}", metadata.display_name, metadata.description);
            for word in text.split_whitespace() {
                let word_lower = word.to_lowercase()
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string();
                if !word_lower.is_empty() {
                    self.search_indices.text_tokens
                        .entry(word_lower)
                        .or_insert_with(Vec::new)
                        .push(normalized_id.clone());
                }
            }
        }
    }

    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();
        let mut results_map: HashMap<String, (SearchResult, f32)> = HashMap::new();
        
        // Search in different indices and accumulate scores
        
        // Exact normalized ID match (highest score)
        for (id, metadata) in &self.normalized_symbols {
            if id.to_lowercase().contains(&query_lower) {
                let score = if id.to_lowercase() == query_lower { 150.0 } else { 120.0 };
                self.add_or_update_result(&mut results_map, id, metadata, score);
            }
        }
        
        // Base currency match
        if let Some(ids) = self.search_indices.by_base.get(&query_lower) {
            for id in ids {
                if let Some(metadata) = self.normalized_symbols.get(id) {
                    self.add_or_update_result(&mut results_map, id, metadata, 100.0);
                }
            }
        }
        
        // Quote currency match
        if let Some(ids) = self.search_indices.by_quote.get(&query_lower) {
            for id in ids {
                if let Some(metadata) = self.normalized_symbols.get(id) {
                    self.add_or_update_result(&mut results_map, id, metadata, 90.0);
                }
            }
        }
        
        // Tag match
        if let Some(ids) = self.search_indices.by_tag.get(&query_lower) {
            for id in ids {
                if let Some(metadata) = self.normalized_symbols.get(id) {
                    self.add_or_update_result(&mut results_map, id, metadata, 80.0);
                }
            }
        }
        
        // Partial matches in tags
        for (tag, ids) in &self.search_indices.by_tag {
            if tag.contains(&query_lower) {
                for id in ids {
                    if let Some(metadata) = self.normalized_symbols.get(id) {
                        self.add_or_update_result(&mut results_map, id, metadata, 60.0);
                    }
                }
            }
        }
        
        // Text token matches
        for (token, ids) in &self.search_indices.text_tokens {
            if token.contains(&query_lower) {
                for id in ids {
                    if let Some(metadata) = self.normalized_symbols.get(id) {
                        let score = if token == &query_lower { 50.0 } else { 40.0 };
                        self.add_or_update_result(&mut results_map, id, metadata, score);
                    }
                }
            }
        }
        
        // Display name partial match
        for (id, metadata) in &self.normalized_symbols {
            if metadata.display_name.to_lowercase().contains(&query_lower) {
                self.add_or_update_result(&mut results_map, id, metadata, 70.0);
            }
        }
        
        // Convert to vector and sort by score
        let mut results: Vec<SearchResult> = results_map
            .into_iter()
            .map(|(_, (mut result, score))| {
                result.relevance_score = score;
                result
            })
            .collect();
        
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        
        // Limit to top 20 results
        results.truncate(20);
        
        results
    }

    fn add_or_update_result(
        &self,
        results_map: &mut HashMap<String, (SearchResult, f32)>,
        normalized_id: &str,
        metadata: &SymbolMetadata,
        score: f32
    ) {
        let exchanges = self.get_exchanges_for_symbol(normalized_id);
        
        if let Some((_, existing_score)) = results_map.get_mut(normalized_id) {
            // Update score if higher
            if score > *existing_score {
                *existing_score = score;
            }
        } else {
            let result = SearchResult {
                normalized_id: normalized_id.to_string(),
                display_name: metadata.display_name.clone(),
                description: metadata.description.clone(),
                base: metadata.base.clone(),
                quote: metadata.quote.clone(),
                category: metadata.category.clone(),
                exchanges,
                relevance_score: score,
            };
            results_map.insert(normalized_id.to_string(), (result, score));
        }
    }

    fn get_exchanges_for_symbol(&self, normalized_id: &str) -> Vec<ExchangeSymbol> {
        let mut exchanges = Vec::new();
        
        for (exchange_name, mappings) in &self.exchange_mappings {
            for (symbol, norm_id) in mappings {
                if norm_id == normalized_id {
                    exchanges.push(ExchangeSymbol {
                        exchange: exchange_name.clone(),
                        symbol: symbol.clone(),
                    });
                }
            }
        }
        
        exchanges
    }
}

// Global instance
pub static SEARCH_SERVICE: once_cell::sync::Lazy<Arc<RwLock<SymbolSearchService>>> =
    once_cell::sync::Lazy::new(|| {
        let mut service = SymbolSearchService::new();
        // Load configs will be called from main.rs
        Arc::new(RwLock::new(service))
    });

// Initialize the search service
pub async fn initialize_search_service() -> Result<(), Box<dyn std::error::Error>> {
    let mut service = SEARCH_SERVICE.write().await;
    service.load_configs()?;
    println!("Symbol search service initialized with {} normalized symbols", 
             service.normalized_symbols.len());
    Ok(())
}