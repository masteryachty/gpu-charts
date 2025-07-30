use anyhow::Result;
use clap::{Parser, Subcommand};
use logger::{Config, Logger};
use std::path::PathBuf;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use warp::Filter;

#[derive(Parser)]
#[command(name = "logger")]
#[command(about = "Multi-exchange cryptocurrency data logger", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Config file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the logger
    Run {
        /// Exchanges to enable (comma-separated: coinbase,binance,okx,kraken,bitfinex)
        #[arg(short, long)]
        exchanges: Option<String>,
    },
    /// Test exchange connections
    Test {
        /// Exchange to test
        exchange: String,
    },
    /// List available symbols for an exchange
    Symbols {
        /// Exchange name
        exchange: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.debug {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = if let Some(config_path) = cli.config {
        Config::from_file(config_path.to_str().unwrap())?
    } else {
        Config::from_env()?
    };

    match cli.command {
        Some(Commands::Run { exchanges }) => {
            run_logger(config, exchanges).await?;
        }
        Some(Commands::Test { exchange }) => {
            test_exchange(&exchange, config).await?;
        }
        Some(Commands::Symbols { exchange }) => {
            list_symbols(&exchange, config).await?;
        }
        None => {
            // Default: run all enabled exchanges
            run_logger(config, None).await?;
        }
    }

    Ok(())
}

async fn run_logger(mut config: Config, exchanges: Option<String>) -> Result<()> {
    // Override enabled exchanges if specified
    if let Some(exchanges_str) = exchanges {
        let exchanges: Vec<&str> = exchanges_str.split(',').collect();

        config.exchanges.coinbase.enabled = exchanges.contains(&"coinbase");
        config.exchanges.binance.enabled = exchanges.contains(&"binance");
        config.exchanges.okx.enabled = exchanges.contains(&"okx");
        config.exchanges.kraken.enabled = exchanges.contains(&"kraken");
        config.exchanges.bitfinex.enabled = exchanges.contains(&"bitfinex");
    }

    info!("Starting multi-exchange logger");
    info!("Enabled exchanges:");
    if config.exchanges.coinbase.enabled {
        info!("  - Coinbase");
    }
    if config.exchanges.binance.enabled {
        info!("  - Binance");
    }
    if config.exchanges.okx.enabled {
        info!("  - OKX");
    }
    if config.exchanges.kraken.enabled {
        info!("  - Kraken");
    }
    if config.exchanges.bitfinex.enabled {
        info!("  - Bitfinex");
    }

    let logger = Logger::new(config.clone())?;

    // Start health check server
    let health_port = config.logger.health_check_port;
    let health_server = tokio::spawn(async move {
        let health = warp::path("health").map(|| {
            warp::reply::json(&serde_json::json!({
                "status": "healthy",
                "service": "multi-exchange-logger"
            }))
        });

        warp::serve(health).run(([0, 0, 0, 0], health_port)).await;
    });

    info!("Health check server running on port {}", health_port);

    // Run logger with graceful shutdown
    tokio::select! {
        result = logger.run() => {
            match result {
                Ok(_) => info!("Logger completed successfully"),
                Err(e) => error!("Logger error: {}", e),
            }
        }
        _ = signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    health_server.abort();
    info!("Shutdown complete");

    Ok(())
}

async fn test_exchange(exchange: &str, config: Config) -> Result<()> {
    use logger::exchanges::Exchange;

    let exchange_impl: Box<dyn Exchange> = match exchange.to_lowercase().as_str() {
        "coinbase" => Box::new(logger::exchanges::coinbase::CoinbaseExchange::new(
            std::sync::Arc::new(config),
        )?),
        "binance" => Box::new(logger::exchanges::binance::BinanceExchange::new(
            std::sync::Arc::new(config),
        )?),
        "okx" => Box::new(logger::exchanges::okx::OkxExchange::new(
            std::sync::Arc::new(config),
        )?),
        "kraken" => Box::new(logger::exchanges::kraken::KrakenExchange::new(
            std::sync::Arc::new(config),
        )?),
        "bitfinex" => Box::new(logger::exchanges::bitfinex::BitfinexExchange::new(
            std::sync::Arc::new(config),
        )?),
        _ => {
            error!("Unknown exchange: {}", exchange);
            return Ok(());
        }
    };

    info!("Testing {} connection...", exchange_impl.name());

    // Test symbol fetching
    match exchange_impl.fetch_symbols().await {
        Ok(symbols) => {
            info!("Successfully fetched {} symbols", symbols.len());
            info!("Sample symbols:");
            for symbol in symbols.iter().take(5) {
                info!("  {} -> {}", symbol.exchange_symbol, symbol.normalized);
            }
        }
        Err(e) => {
            error!("Failed to fetch symbols: {}", e);
        }
    }

    Ok(())
}

async fn list_symbols(exchange: &str, config: Config) -> Result<()> {
    use logger::exchanges::Exchange;

    let exchange_impl: Box<dyn Exchange> = match exchange.to_lowercase().as_str() {
        "coinbase" => Box::new(logger::exchanges::coinbase::CoinbaseExchange::new(
            std::sync::Arc::new(config),
        )?),
        "binance" => Box::new(logger::exchanges::binance::BinanceExchange::new(
            std::sync::Arc::new(config),
        )?),
        "okx" => Box::new(logger::exchanges::okx::OkxExchange::new(
            std::sync::Arc::new(config),
        )?),
        "kraken" => Box::new(logger::exchanges::kraken::KrakenExchange::new(
            std::sync::Arc::new(config),
        )?),
        "bitfinex" => Box::new(logger::exchanges::bitfinex::BitfinexExchange::new(
            std::sync::Arc::new(config),
        )?),
        _ => {
            error!("Unknown exchange: {}", exchange);
            return Ok(());
        }
    };

    info!("Fetching symbols from {}...", exchange_impl.name());

    let symbols = exchange_impl.fetch_symbols().await?;

    println!("Exchange,Symbol,Normalized,Base,Quote,Status");
    for symbol in &symbols {
        println!(
            "{},{},{},{},{},{}",
            exchange,
            symbol.exchange_symbol,
            symbol.normalized,
            symbol.base_asset,
            symbol.quote_asset,
            if symbol.active { "active" } else { "inactive" }
        );
    }

    info!("Total symbols: {}", symbols.len());

    Ok(())
}
