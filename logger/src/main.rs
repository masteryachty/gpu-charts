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
    let mut config = if let Some(config_path) = cli.config.as_deref() {
        Config::from_file(config_path.to_str().unwrap_or("config.yaml"))?
    } else {
        Config::from_env()?
    };

    // Handle commands
    match cli.command {
        Some(Commands::Run { exchanges }) => {
            if let Some(exchanges) = exchanges {
                // Override config with command line exchanges
                let exchange_list: Vec<String> = exchanges.split(',').map(|s| s.trim().to_string()).collect();
                // Disable all exchanges first
                config.exchanges.coinbase.enabled = false;
                config.exchanges.binance.enabled = false;
                config.exchanges.okx.enabled = false;
                config.exchanges.kraken.enabled = false;
                config.exchanges.bitfinex.enabled = false;
                
                // Enable only the specified exchanges
                for exchange in exchange_list {
                    match exchange.to_lowercase().as_str() {
                        "coinbase" => config.exchanges.coinbase.enabled = true,
                        "binance" => config.exchanges.binance.enabled = true,
                        "okx" => config.exchanges.okx.enabled = true,
                        "kraken" => config.exchanges.kraken.enabled = true,
                        "bitfinex" => config.exchanges.bitfinex.enabled = true,
                        _ => error!("Unknown exchange: {}", exchange),
                    }
                }
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

async fn test_exchange(exchange_name: &str, _config: &Config) -> Result<()> {
    info!("Testing connection to {}...", exchange_name);
    
    // TODO: Implement exchange testing
    error!("Exchange testing not yet implemented for {}", exchange_name);
    
    Ok(())
}

async fn list_symbols(exchange_name: &str, _config: &Config) -> Result<()> {
    info!("Listing symbols for {}...", exchange_name);
    
    // TODO: Implement symbol listing
    error!("Symbol listing not yet implemented for {}", exchange_name);
    
    Ok(())
}