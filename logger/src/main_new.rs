use anyhow::Result;
use clap::{Parser, Subcommand};
use logger::{Config, Logger};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod metrics_server;

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
    
    // Check if we're running under systemd/Docker (they add timestamps)
    let is_systemd = std::env::var("INVOCATION_ID").is_ok() || std::env::var("JOURNAL_STREAM").is_ok();
    let is_docker = std::path::Path::new("/.dockerenv").exists();
    
    if is_systemd || is_docker {
        // Use compact format without timestamps for systemd/Docker
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(log_level)))
            .with_target(false)
            .without_time()
            .init();
    } else {
        // Use full format with timestamps for direct execution
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(log_level)))
            .with_target(false)
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
            test_exchange(&exchange).await?;
        }
        Some(Commands::Symbols { exchange }) => {
            list_symbols(&exchange).await?;
        }
        None => {
            // Default to run command
            run_logger(config).await?;
        }
    }

    Ok(())
}

async fn run_logger(config: Config) -> Result<()> {
    info!("Starting logger with configuration");
    
    // Start metrics server on configured port (default 9090)
    let metrics_port = std::env::var("METRICS_PORT")
        .unwrap_or_else(|_| "9090".to_string())
        .parse::<u16>()
        .unwrap_or(9090);
    
    tokio::spawn(async move {
        metrics_server::start_metrics_server(metrics_port).await;
    });
    
    info!("Metrics server started on port {}", metrics_port);
    info!("Prometheus can scrape metrics from http://localhost:{}/metrics", metrics_port);
    
    // Create and start logger
    let logger = Arc::new(Logger::new(config));
    let logger_clone = Arc::clone(&logger);

    // Spawn logger task
    let logger_handle = tokio::spawn(async move {
        if let Err(e) = logger_clone.run().await {
            error!("Logger error: {}", e);
        }
    });

    // Wait for shutdown signal
    shutdown_signal().await;
    info!("Shutdown signal received, stopping logger...");

    // The logger should handle graceful shutdown internally
    // Just wait for it to complete
    let _ = logger_handle.await;

    info!("Logger stopped successfully");
    Ok(())
}

async fn test_exchange(exchange_name: &str) -> Result<()> {
    info!("Testing connection to {}", exchange_name);
    // TODO: Implement exchange connection test
    info!("Exchange test not yet implemented");
    Ok(())
}

async fn list_symbols(exchange_name: &str) -> Result<()> {
    info!("Fetching symbols for {}", exchange_name);
    // TODO: Implement symbol listing
    info!("Symbol listing not yet implemented");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}