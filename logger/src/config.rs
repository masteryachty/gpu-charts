use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub logger: LoggerConfig,
    pub exchanges: ExchangesConfig,
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
    pub okx: ExchangeConfig,
    pub kraken: ExchangeConfig,
    pub bitfinex: ExchangeConfig,
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
                    ping_interval_secs: None, // Coinbase uses subscription-based heartbeats
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
                okx: ExchangeConfig {
                    enabled: true,
                    ws_endpoint: "wss://ws.okx.com:8443/ws/v5/public".to_string(),
                    rest_endpoint: "https://www.okx.com/api/v5".to_string(),
                    max_connections: 10,
                    symbols_per_connection: 100,
                    reconnect_delay_secs: 1,
                    max_reconnect_delay_secs: 60,
                    ping_interval_secs: Some(30), // OKX requires pings every 30s
                    symbols: None,
                },
                kraken: ExchangeConfig {
                    enabled: true,
                    ws_endpoint: "wss://ws.kraken.com".to_string(),
                    rest_endpoint: "https://api.kraken.com/0".to_string(),
                    max_connections: 5,
                    symbols_per_connection: 50,
                    reconnect_delay_secs: 1,
                    max_reconnect_delay_secs: 60,
                    ping_interval_secs: Some(60), // Kraken heartbeat interval
                    symbols: None,
                },
                bitfinex: ExchangeConfig {
                    enabled: true,
                    ws_endpoint: "wss://api-pub.bitfinex.com/ws/2".to_string(),
                    rest_endpoint: "https://api-pub.bitfinex.com/v2".to_string(),
                    max_connections: 10,
                    symbols_per_connection: 15, // Bitfinex has a limit on subscriptions per connection
                    reconnect_delay_secs: 1,
                    max_reconnect_delay_secs: 60,
                    ping_interval_secs: Some(15), // Bitfinex requires pings every 15s
                    symbols: None,
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
        // Try to load from default config file location first
        let default_config_path = "/home/logger/config.yaml";
        let settings = if std::path::Path::new(default_config_path).exists() {
            config::Config::builder()
                .add_source(config::File::with_name(default_config_path))
                .add_source(config::Environment::with_prefix("LOGGER"))
                .build()?
        } else {
            // Fall back to hardcoded defaults
            config::Config::builder()
                .add_source(config::Config::try_from(&Config::default())?)
                .add_source(config::Environment::with_prefix("LOGGER"))
                .build()?
        };

        Ok(settings.try_deserialize()?)
    }
}
