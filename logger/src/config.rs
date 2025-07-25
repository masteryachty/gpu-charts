use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub logger: LoggerConfig,
    pub exchanges: ExchangesConfig,
    pub symbol_mappings: SymbolMappingsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggerConfig {
    pub data_path: PathBuf,
    pub buffer_size: usize,
    pub flush_interval_secs: u64,
    pub health_check_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangesConfig {
    pub coinbase: ExchangeConfig,
    pub binance: ExchangeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub enabled: bool,
    pub ws_endpoint: String,
    pub rest_endpoint: String,
    pub max_connections: usize,
    pub symbols_per_connection: usize,
    pub reconnect_delay_secs: u64,
    pub max_reconnect_delay_secs: u64,
    pub ping_interval_secs: Option<u64>,
    pub symbols: Option<Vec<String>>, // If None, fetch all available
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMappingsConfig {
    pub mappings_file: Option<PathBuf>,
    pub auto_discover: bool,
    pub equivalence_rules: EquivalenceRules,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquivalenceRules {
    pub quote_assets: Vec<AssetGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGroup {
    pub group: String,
    pub members: Vec<String>,
    pub primary: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            logger: LoggerConfig {
                data_path: PathBuf::from("/mnt/md/data"),
                buffer_size: 8192,
                flush_interval_secs: 5,
                health_check_port: 8080,
            },
            exchanges: ExchangesConfig {
                coinbase: ExchangeConfig {
                    enabled: true,
                    ws_endpoint: "wss://ws-feed.exchange.coinbase.com".to_string(),
                    rest_endpoint: "https://api.exchange.coinbase.com".to_string(),
                    max_connections: 10,
                    symbols_per_connection: 50,
                    reconnect_delay_secs: 1,
                    max_reconnect_delay_secs: 60,
                    ping_interval_secs: None, // Coinbase doesn't require client pings
                    symbols: None,
                },
                binance: ExchangeConfig {
                    enabled: true,
                    ws_endpoint: "wss://stream.binance.com:9443".to_string(),
                    rest_endpoint: "https://api.binance.com".to_string(),
                    max_connections: 5,
                    symbols_per_connection: 100,
                    reconnect_delay_secs: 1,
                    max_reconnect_delay_secs: 60,
                    ping_interval_secs: Some(20), // Binance requires pings every 20s
                    symbols: None,
                },
            },
            symbol_mappings: SymbolMappingsConfig {
                mappings_file: Some(PathBuf::from("symbol_mappings.yaml")),
                auto_discover: true,
                equivalence_rules: EquivalenceRules {
                    quote_assets: vec![AssetGroup {
                        group: "USD_EQUIVALENT".to_string(),
                        members: vec![
                            "USD".to_string(),
                            "USDT".to_string(),
                            "USDC".to_string(),
                            "BUSD".to_string(),
                            "DAI".to_string(),
                        ],
                        primary: "USD".to_string(),
                    }],
                },
            },
        }
    }
}

impl Config {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name(path))
            .add_source(config::Environment::with_prefix("LOGGER"))
            .build()?;

        Ok(settings.try_deserialize()?)
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let settings = config::Config::builder()
            .add_source(config::Config::try_from(&Config::default())?)
            .add_source(config::Environment::with_prefix("LOGGER"))
            .build()?;

        Ok(settings.try_deserialize()?)
    }
}
