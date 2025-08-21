use anyhow::Result;
use clap::{Parser, Subcommand};
use logger::{Config, Logger};
use std::path::PathBuf;
use std::sync::Arc;
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

    // Initialize tracing
    let log_level = if cli.debug { "debug" } else { "info" };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));
    
    // Check if we're running under systemd/docker (they add their own timestamps)
    let is_systemd = std::env::var("INVOCATION_ID").is_ok() || std::env::var("JOURNAL_STREAM").is_ok();
    let is_docker = std::path::Path::new("/.dockerenv").exists();
    
    // Configure the fmt layer based on environment
    if is_systemd || is_docker {
        // Omit timestamp when running under systemd/docker as they add their own
        tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .without_time()
                    .compact()
            )
            .init();
    } else {
        // Include timestamp for local development
        tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .compact()
            )
            .init();
    }

    // Load configuration
    let config_path = cli.config.as_deref();
    let mut config = Config::load(config_path)?;

    // Handle commands
    match cli.command {
        Some(Commands::Run { exchanges }) => {
            if let Some(exchanges) = exchanges {
                // Override config with command line exchanges
                let exchange_list: Vec<String> = exchanges.split(',').map(|s| s.trim().to_string()).collect();
                config.enable_only_exchanges(&exchange_list);
            }
            run_logger(config).await?;
        }
        Some(Commands::Test { exchange }) => {
            test_exchange(&exchange, &config).await?;
        }
        Some(Commands::Symbols { exchange }) => {
            list_symbols(&exchange, &config).await?;
        }
        None => {
            // Default to run command
            run_logger(config).await?;
        }
    }

    Ok(())
}

async fn run_logger(config: Config) -> Result<()> {
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

    // Start metrics server on configured port (default 9090)
    let metrics_port = std::env::var("METRICS_PORT")
        .unwrap_or_else(|_| "9090".to_string())
        .parse::<u16>()
        .unwrap_or(9090);
    
    tokio::spawn(async move {
        logger::metrics_server::start_metrics_server(metrics_port).await;
    });
    
    info!("Metrics server started on port {}", metrics_port);
    info!("Prometheus can scrape metrics from http://localhost:{}/metrics", metrics_port);

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
    info!("Logger shutdown complete");

    Ok(())
}

async fn test_exchange(exchange_name: &str, config: &Config) -> Result<()> {
    use logger::exchanges::{Exchange, ExchangeFactory};

    info!("Testing connection to {}...", exchange_name);

    let factory = ExchangeFactory::new();
    let exchange = factory.create(exchange_name, config.clone())?;

    // Test fetching symbols
    info!("Fetching symbols...");
    match exchange.fetch_symbols().await {
        Ok(symbols) => {
            info!("Successfully fetched {} symbols", symbols.len());
            if symbols.len() > 5 {
                info!("Sample symbols:");
                for symbol in symbols.iter().take(5) {
                    info!("  - {}", symbol.symbol);
                }
                info!("  ... and {} more", symbols.len() - 5);
            } else {
                for symbol in &symbols {
                    info!("  - {}", symbol.symbol);
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch symbols: {}", e);
            return Err(e);
        }
    }

    // Test creating a connection
    info!("Testing WebSocket connection...");
    let channels = vec![];
    match exchange.create_connection(channels).await {
        Ok(mut conn) => {
            match conn.connect().await {
                Ok(_) => {
                    info!("Successfully connected to WebSocket");
                    info!("Connection test passed!");
                }
                Err(e) => {
                    error!("Failed to connect: {}", e);
                    return Err(e);
                }
            }
        }
        Err(e) => {
            error!("Failed to create connection: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

async fn list_symbols(exchange_name: &str, config: &Config) -> Result<()> {
    use logger::exchanges::{Exchange, ExchangeFactory};

    let factory = ExchangeFactory::new();
    let exchange = factory.create(exchange_name, config.clone())?;

    info!("Fetching symbols for {}...", exchange_name);
    let symbols = exchange.fetch_symbols().await?;

    info!("Found {} symbols:", symbols.len());
    for symbol in symbols {
        let active_str = if symbol.active { "active" } else { "inactive" };
        let tick_str = symbol
            .tick_size
            .map(|t| format!("{}", t))
            .unwrap_or_else(|| "N/A".to_string());
        let min_str = symbol
            .min_size
            .map(|m| format!("{}", m))
            .unwrap_or_else(|| "N/A".to_string());

        info!(
            "  {} - {}/{} ({}) [tick: {}, min: {}]",
            symbol.symbol,
            symbol.base_asset,
            symbol.quote_asset,
            active_str,
            tick_str,
            min_str
        );
    }

    info!("Total symbols: {}", symbols.len());

    Ok(())
}