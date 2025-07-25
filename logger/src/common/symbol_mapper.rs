use crate::common::data_types::{AssetClass, ExchangeId, QuoteType, Symbol};
use crate::config::SymbolMappingsConfig;
use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SymbolMapper {
    mappings: Arc<DashMap<String, ExchangeSymbolMap>>,
    normalized_index: Arc<DashMap<String, String>>, // exchange:symbol -> normalized
    asset_groups: Arc<DashMap<String, Vec<SymbolInfo>>>, // BTC -> all BTC pairs
    config: SymbolMappingsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeSymbolMap {
    pub normalized: String,
    pub exchange_symbols: HashMap<ExchangeId, String>,
    pub asset_class: AssetClass,
    pub base_asset: String,
    pub quote_asset: String,
    pub quote_type: QuoteType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub exchange: ExchangeId,
    pub symbol: String,
    pub normalized: String,
    pub active: bool,
    pub min_size: Option<f64>,
    pub tick_size: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolMappingFile {
    pub symbol_mappings: Vec<SymbolMappingEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolMappingEntry {
    pub normalized: String,
    pub base: String,
    pub quote: String,
    pub quote_type: String,
    pub exchanges: HashMap<String, String>,
}

impl SymbolMapper {
    pub fn new(config: SymbolMappingsConfig) -> Result<Self> {
        let mapper = Self {
            mappings: Arc::new(DashMap::new()),
            normalized_index: Arc::new(DashMap::new()),
            asset_groups: Arc::new(DashMap::new()),
            config,
        };

        // Load mappings from file if specified
        if let Some(ref path) = mapper.config.mappings_file {
            if Path::new(path).exists() {
                mapper.load_from_file(path)?;
            }
        }

        Ok(mapper)
    }

    pub fn load_from_file(&self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let mapping_file: SymbolMappingFile = serde_yaml::from_str(&content)?;

        for entry in mapping_file.symbol_mappings {
            let quote_type = match entry.quote_type.to_lowercase().as_str() {
                "fiat" => QuoteType::Fiat(entry.quote.clone()),
                "stablecoin" => QuoteType::Stablecoin(entry.quote.clone()),
                "crypto" => QuoteType::Crypto(entry.quote.clone()),
                _ => QuoteType::Fiat(entry.quote.clone()),
            };

            let mut exchange_symbols = HashMap::new();
            for (exchange_str, symbol) in entry.exchanges {
                if let Some(exchange_id) = Self::parse_exchange_id(&exchange_str) {
                    exchange_symbols.insert(exchange_id, symbol.clone());

                    // Add to normalized index
                    let key = format!("{}:{}", exchange_id.as_str(), symbol);
                    self.normalized_index.insert(key, entry.normalized.clone());
                }
            }

            let map = ExchangeSymbolMap {
                normalized: entry.normalized.clone(),
                exchange_symbols,
                asset_class: AssetClass::Spot, // Default to spot
                base_asset: entry.base.clone(),
                quote_asset: entry.quote.clone(),
                quote_type,
            };

            self.mappings.insert(entry.normalized.clone(), map);
        }

        Ok(())
    }

    fn parse_exchange_id(s: &str) -> Option<ExchangeId> {
        match s.to_lowercase().as_str() {
            "coinbase" => Some(ExchangeId::Coinbase),
            "binance" => Some(ExchangeId::Binance),
            "kraken" => Some(ExchangeId::Kraken),
            "bybit" => Some(ExchangeId::Bybit),
            _ => None,
        }
    }

    pub fn normalize(&self, exchange: ExchangeId, symbol: &str) -> Option<String> {
        let key = format!("{}:{}", exchange.as_str(), symbol);
        self.normalized_index.get(&key).map(|v| v.clone())
    }

    pub fn to_exchange(&self, normalized: &str, exchange: ExchangeId) -> Option<String> {
        self.mappings
            .get(normalized)
            .and_then(|map| map.exchange_symbols.get(&exchange).cloned())
    }

    pub fn find_related(&self, base: &str, quote: &str) -> Vec<SymbolInfo> {
        let mut results = Vec::new();

        for entry in self.mappings.iter() {
            let map = entry.value();
            if map.base_asset == base && map.quote_asset == quote {
                for (exchange, symbol) in &map.exchange_symbols {
                    results.push(SymbolInfo {
                        exchange: *exchange,
                        symbol: symbol.clone(),
                        normalized: map.normalized.clone(),
                        active: true,
                        min_size: None,
                        tick_size: None,
                    });
                }
            }
        }

        results
    }

    pub fn get_usd_pairs(&self, asset: &str) -> Vec<SymbolInfo> {
        let mut results = Vec::new();
        let usd_equivalents = &self.config.equivalence_rules.quote_assets[0].members;

        for entry in self.mappings.iter() {
            let map = entry.value();
            if map.base_asset == asset {
                let quote = &map.quote_asset;
                if usd_equivalents.contains(quote) {
                    for (exchange, symbol) in &map.exchange_symbols {
                        results.push(SymbolInfo {
                            exchange: *exchange,
                            symbol: symbol.clone(),
                            normalized: map.normalized.clone(),
                            active: true,
                            min_size: None,
                            tick_size: None,
                        });
                    }
                }
            }
        }

        results
    }

    pub fn are_equivalent(&self, sym1: &str, sym2: &str) -> bool {
        if sym1 == sym2 {
            return true;
        }

        // Check if both symbols map to the same normalized form
        let norm1 = self.mappings.get(sym1).map(|m| m.normalized.clone());
        let norm2 = self.mappings.get(sym2).map(|m| m.normalized.clone());

        if let (Some(n1), Some(n2)) = (norm1, norm2) {
            if n1 == n2 {
                return true;
            }
        }

        // Check if they have same base asset and equivalent quote assets
        if let (Some(map1), Some(map2)) = (self.mappings.get(sym1), self.mappings.get(sym2)) {
            if map1.base_asset == map2.base_asset {
                // Check if quotes are in same equivalence group
                for group in &self.config.equivalence_rules.quote_assets {
                    if group.members.contains(&map1.quote_asset)
                        && group.members.contains(&map2.quote_asset)
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn add_symbol(&self, symbol: Symbol) {
        let key = format!("{}:{}", symbol.exchange.as_str(), &symbol.exchange_symbol);
        self.normalized_index.insert(key, symbol.normalized.clone());

        // Update or create mapping
        self.mappings
            .entry(symbol.normalized.clone())
            .and_modify(|map| {
                map.exchange_symbols
                    .insert(symbol.exchange, symbol.exchange_symbol.clone());
            })
            .or_insert_with(|| {
                let mut exchange_symbols = HashMap::new();
                exchange_symbols.insert(symbol.exchange, symbol.exchange_symbol.clone());

                let quote_type = if ["USD", "EUR", "GBP", "JPY"]
                    .contains(&symbol.quote_asset.as_str())
                {
                    QuoteType::Fiat(symbol.quote_asset.clone())
                } else if ["USDT", "USDC", "BUSD", "DAI"].contains(&symbol.quote_asset.as_str()) {
                    QuoteType::Stablecoin(symbol.quote_asset.clone())
                } else {
                    QuoteType::Crypto(symbol.quote_asset.clone())
                };

                ExchangeSymbolMap {
                    normalized: symbol.normalized.clone(),
                    exchange_symbols,
                    asset_class: symbol.asset_class,
                    base_asset: symbol.base_asset.clone(),
                    quote_asset: symbol.quote_asset.clone(),
                    quote_type,
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AssetGroup;
    use crate::config::EquivalenceRules;

    fn create_test_mapper() -> SymbolMapper {
        let config = SymbolMappingsConfig {
            mappings_file: None,
            auto_discover: true,
            equivalence_rules: EquivalenceRules {
                quote_assets: vec![AssetGroup {
                    group: "USD_EQUIVALENT".to_string(),
                    members: vec!["USD".to_string(), "USDT".to_string(), "USDC".to_string()],
                    primary: "USD".to_string(),
                }],
            },
        };

        let mapper = SymbolMapper::new(config).unwrap();

        // Add test symbols
        mapper.add_symbol(Symbol {
            exchange: ExchangeId::Coinbase,
            exchange_symbol: "BTC-USD".to_string(),
            normalized: "BTC-USD".to_string(),
            base_asset: "BTC".to_string(),
            quote_asset: "USD".to_string(),
            asset_class: AssetClass::Spot,
            active: true,
            min_size: None,
            tick_size: None,
        });

        mapper.add_symbol(Symbol {
            exchange: ExchangeId::Binance,
            exchange_symbol: "BTCUSDT".to_string(),
            normalized: "BTC-USDT".to_string(),
            base_asset: "BTC".to_string(),
            quote_asset: "USDT".to_string(),
            asset_class: AssetClass::Spot,
            active: true,
            min_size: None,
            tick_size: None,
        });

        mapper
    }

    #[test]
    fn test_symbol_normalization() {
        let mapper = create_test_mapper();

        assert_eq!(
            mapper.normalize(ExchangeId::Coinbase, "BTC-USD"),
            Some("BTC-USD".to_string())
        );

        assert_eq!(
            mapper.normalize(ExchangeId::Binance, "BTCUSDT"),
            Some("BTC-USDT".to_string())
        );
    }

    #[test]
    fn test_symbol_to_exchange() {
        let mapper = create_test_mapper();

        assert_eq!(
            mapper.to_exchange("BTC-USD", ExchangeId::Coinbase),
            Some("BTC-USD".to_string())
        );

        assert_eq!(
            mapper.to_exchange("BTC-USDT", ExchangeId::Binance),
            Some("BTCUSDT".to_string())
        );
    }

    #[test]
    fn test_find_usd_pairs() {
        let mapper = create_test_mapper();
        let pairs = mapper.get_usd_pairs("BTC");

        assert_eq!(pairs.len(), 2);
        assert!(pairs.iter().any(|p| p.symbol == "BTC-USD"));
        assert!(pairs.iter().any(|p| p.symbol == "BTCUSDT"));
    }

    #[test]
    fn test_symbol_equivalence() {
        let mapper = create_test_mapper();

        // Same symbol
        assert!(mapper.are_equivalent("BTC-USD", "BTC-USD"));

        // Different but equivalent (same base, equivalent quote)
        assert!(mapper.are_equivalent("BTC-USD", "BTC-USDT"));

        // Not equivalent
        mapper.add_symbol(Symbol {
            exchange: ExchangeId::Coinbase,
            exchange_symbol: "ETH-USD".to_string(),
            normalized: "ETH-USD".to_string(),
            base_asset: "ETH".to_string(),
            quote_asset: "USD".to_string(),
            asset_class: AssetClass::Spot,
            active: true,
            min_size: None,
            tick_size: None,
        });

        assert!(!mapper.are_equivalent("BTC-USD", "ETH-USD"));
    }
}
