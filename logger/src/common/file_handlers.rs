use crate::common::data_types::{ExchangeId, UnifiedMarketData, UnifiedTradeData};
use anyhow::{Context, Result};
use byteorder::{LittleEndian, WriteBytesExt};
use chrono::{DateTime, Datelike, Utc};
use dashmap::DashMap;
use std::collections::BTreeMap;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;
use tracing::warn;

const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer

#[derive(Debug)]
pub struct FileHandlerManager {
    base_path: PathBuf,
    handlers: Arc<DashMap<String, Arc<Mutex<FileHandlers>>>>,
}

#[derive(Debug)]
pub struct FileHandlers {
    _exchange: ExchangeId,
    _symbol: String,
    date: DateTime<Utc>,
    md_files: MarketDataFiles,
    trade_files: TradeFiles,
}

#[derive(Debug)]
struct MarketDataFiles {
    time: BufWriter<std::fs::File>,
    nanos: BufWriter<std::fs::File>,
    price: BufWriter<std::fs::File>,
    volume: BufWriter<std::fs::File>,
    side: BufWriter<std::fs::File>,
    best_bid: BufWriter<std::fs::File>,
    best_ask: BufWriter<std::fs::File>,
}

#[derive(Debug)]
struct TradeFiles {
    trade_id: BufWriter<std::fs::File>,
    trade_time: BufWriter<std::fs::File>,
    trade_nanos: BufWriter<std::fs::File>,
    trade_price: BufWriter<std::fs::File>,
    trade_size: BufWriter<std::fs::File>,
    trade_side: BufWriter<std::fs::File>,
    maker_order_id: BufWriter<std::fs::File>,
    taker_order_id: BufWriter<std::fs::File>,
}

impl FileHandlerManager {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            handlers: Arc::new(DashMap::new()),
        }
    }

    pub async fn get_or_create(
        &self,
        exchange: ExchangeId,
        symbol: &str,
        date: DateTime<Utc>,
    ) -> Result<Arc<Mutex<FileHandlers>>> {
        let key = format!(
            "{}:{}:{}",
            exchange.as_str(),
            symbol,
            date.format("%Y-%m-%d")
        );

        if let Some(handler) = self.handlers.get(&key) {
            return Ok(handler.clone());
        }

        let handler = Arc::new(Mutex::new(
            FileHandlers::new(&self.base_path, exchange, symbol.to_string(), date).await?,
        ));

        self.handlers.insert(key, handler.clone());
        Ok(handler)
    }

    pub async fn flush_all(&self) -> Result<()> {
        for entry in self.handlers.iter() {
            let handler = entry.value();
            let mut handler = handler.lock().await;
            handler.flush().await?;
        }
        Ok(())
    }

    pub async fn rotate_if_needed(&self) -> Result<()> {
        let now = Utc::now();
        let mut to_remove = Vec::new();

        for entry in self.handlers.iter() {
            let key = entry.key().clone();
            let handler = entry.value();
            let handler_guard = handler.lock().await;

            if handler_guard.date.date_naive() != now.date_naive() {
                to_remove.push(key);
            }
        }

        for key in to_remove {
            if let Some((_, handler)) = self.handlers.remove(&key) {
                let mut handler = handler.lock().await;
                handler.flush().await?;
            }
        }

        Ok(())
    }
}

impl FileHandlers {
    async fn new(
        base_path: &Path,
        exchange: ExchangeId,
        symbol: String,
        date: DateTime<Utc>,
    ) -> Result<Self> {
        let exchange_path = base_path.join(exchange.as_str());
        // Sanitize symbol for filesystem: replace / with _
        let sanitized_symbol = symbol.replace('/', "_");
        let symbol_path = exchange_path.join(&sanitized_symbol);

        let md_path = symbol_path.join("MD");
        let trades_path = symbol_path.join("TRADES");

        // Create directories
        fs::create_dir_all(&md_path)
            .await
            .with_context(|| format!("Failed to create MD directory: {md_path:?}"))?;
        fs::create_dir_all(&trades_path)
            .await
            .with_context(|| format!("Failed to create TRADES directory: {trades_path:?}"))?;

        let date_suffix = format!(
            "{:02}.{:02}.{:02}",
            date.day(),
            date.month(),
            date.year() % 100
        );

        // Create market data files
        let md_files = MarketDataFiles {
            time: Self::create_file(&md_path, "time", &date_suffix)?,
            nanos: Self::create_file(&md_path, "nanos", &date_suffix)?,
            price: Self::create_file(&md_path, "price", &date_suffix)?,
            volume: Self::create_file(&md_path, "volume", &date_suffix)?,
            side: Self::create_file(&md_path, "side", &date_suffix)?,
            best_bid: Self::create_file(&md_path, "best_bid", &date_suffix)?,
            best_ask: Self::create_file(&md_path, "best_ask", &date_suffix)?,
        };

        // Create trade files
        let trade_files = TradeFiles {
            trade_id: Self::create_file(&trades_path, "id", &date_suffix)?,
            trade_time: Self::create_file(&trades_path, "time", &date_suffix)?,
            trade_nanos: Self::create_file(&trades_path, "nanos", &date_suffix)?,
            trade_price: Self::create_file(&trades_path, "price", &date_suffix)?,
            trade_size: Self::create_file(&trades_path, "size", &date_suffix)?,
            trade_side: Self::create_file(&trades_path, "side", &date_suffix)?,
            maker_order_id: Self::create_file(&trades_path, "maker_order_id", &date_suffix)?,
            taker_order_id: Self::create_file(&trades_path, "taker_order_id", &date_suffix)?,
        };

        Ok(Self {
            _exchange: exchange,
            _symbol: symbol,
            date,
            md_files,
            trade_files,
        })
    }

    fn create_file(path: &Path, name: &str, date_suffix: &str) -> Result<BufWriter<std::fs::File>> {
        let filename = format!("{name}.{date_suffix}.bin");
        let file_path = path.join(filename);

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .with_context(|| format!("Failed to open file: {file_path:?}"))?;

        Ok(BufWriter::with_capacity(BUFFER_SIZE, file))
    }

    pub async fn write_market_data(&mut self, data: &UnifiedMarketData) -> Result<()> {
        self.md_files
            .time
            .write_u32::<LittleEndian>(data.timestamp)?;
        self.md_files.nanos.write_u32::<LittleEndian>(data.nanos)?;
        self.md_files.price.write_f32::<LittleEndian>(data.price)?;
        self.md_files
            .volume
            .write_f32::<LittleEndian>(data.volume)?;
        self.md_files
            .side
            .write_u32::<LittleEndian>(data.side.as_u32())?;
        self.md_files
            .best_bid
            .write_f32::<LittleEndian>(data.best_bid)?;
        self.md_files
            .best_ask
            .write_f32::<LittleEndian>(data.best_ask)?;

        Ok(())
    }

    pub async fn write_trade_data(&mut self, data: &UnifiedTradeData) -> Result<()> {
        self.trade_files
            .trade_id
            .write_u64::<LittleEndian>(data.trade_id)?;
        self.trade_files
            .trade_time
            .write_u32::<LittleEndian>(data.timestamp)?;
        self.trade_files
            .trade_nanos
            .write_u32::<LittleEndian>(data.nanos)?;
        self.trade_files
            .trade_price
            .write_f32::<LittleEndian>(data.price)?;
        self.trade_files
            .trade_size
            .write_f32::<LittleEndian>(data.size)?;
        self.trade_files
            .trade_side
            .write_u32::<LittleEndian>(data.side.as_u32())?;
        self.trade_files
            .maker_order_id
            .write_all(&data.maker_order_id)?;
        self.trade_files
            .taker_order_id
            .write_all(&data.taker_order_id)?;

        Ok(())
    }

    pub async fn flush(&mut self) -> Result<()> {
        use std::time::Duration;
        use tokio::time::sleep;

        const MAX_RETRIES: u32 = 3;
        let mut retries = 0;
        let mut delay = Duration::from_millis(100);

        loop {
            let result = self.flush_all_files();

            match result {
                Ok(_) => {
                    if retries > 0 {
                        warn!("File flush succeeded after {} retries", retries);
                    }
                    return Ok(());
                }
                Err(e) => {
                    // Check if this is an I/O error (os error 5)
                    let error_string = e.to_string();
                    let is_io_error = error_string.contains("Input/output error")
                        || error_string.contains("os error 5");

                    if is_io_error && retries < MAX_RETRIES {
                        warn!(
                            "I/O error during flush (attempt {}/{}), retrying in {:?}: {}",
                            retries + 1,
                            MAX_RETRIES,
                            delay,
                            e
                        );
                        sleep(delay).await;
                        delay = delay.saturating_mul(2); // Exponential backoff
                        retries += 1;
                    } else {
                        // For non-I/O errors or after max retries, return the error
                        return Err(anyhow::anyhow!(
                            "Failed to flush files after {} attempts: {}",
                            retries + 1,
                            e
                        ));
                    }
                }
            }
        }
    }

    fn flush_all_files(&mut self) -> Result<()> {
        let mut errors = Vec::new();
        let mut successful = 0;

        // Flush market data files
        let md_flushes = [
            (&mut self.md_files.time, "md_time"),
            (&mut self.md_files.nanos, "md_nanos"),
            (&mut self.md_files.price, "md_price"),
            (&mut self.md_files.volume, "md_volume"),
            (&mut self.md_files.side, "md_side"),
            (&mut self.md_files.best_bid, "md_best_bid"),
            (&mut self.md_files.best_ask, "md_best_ask"),
        ];

        for (file, name) in md_flushes {
            match file.flush() {
                Ok(_) => successful += 1,
                Err(e) => {
                    warn!("Failed to flush {}: {}", name, e);
                    errors.push((name, e));
                }
            }
        }

        // Flush trade files
        let trade_flushes = [
            (&mut self.trade_files.trade_id, "trade_id"),
            (&mut self.trade_files.trade_time, "trade_time"),
            (&mut self.trade_files.trade_nanos, "trade_nanos"),
            (&mut self.trade_files.trade_price, "trade_price"),
            (&mut self.trade_files.trade_size, "trade_size"),
            (&mut self.trade_files.trade_side, "trade_side"),
            (&mut self.trade_files.maker_order_id, "maker_order_id"),
            (&mut self.trade_files.taker_order_id, "taker_order_id"),
        ];

        for (file, name) in trade_flushes {
            match file.flush() {
                Ok(_) => successful += 1,
                Err(e) => {
                    warn!("Failed to flush {}: {}", name, e);
                    errors.push((name, e));
                }
            }
        }

        // If all files failed, return error
        if successful == 0 && !errors.is_empty() {
            return Err(anyhow::anyhow!("All flush operations failed"));
        }

        // If some succeeded, log warning but continue
        if !errors.is_empty() {
            warn!(
                "Partial flush success: {}/{} files flushed. Failed: {:?}",
                successful,
                successful + errors.len(),
                errors.iter().map(|(name, _)| name).collect::<Vec<_>>()
            );
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct DataBuffer {
    market_data: Arc<Mutex<BTreeMap<(u32, u32), UnifiedMarketData>>>,
    trade_data: Arc<Mutex<BTreeMap<u64, UnifiedTradeData>>>,
    file_manager: Arc<FileHandlerManager>,
}

impl DataBuffer {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            market_data: Arc::new(Mutex::new(BTreeMap::new())),
            trade_data: Arc::new(Mutex::new(BTreeMap::new())),
            file_manager: Arc::new(FileHandlerManager::new(base_path)),
        }
    }

    pub async fn add_market_data(&self, data: UnifiedMarketData) -> Result<()> {
        let mut buffer = self.market_data.lock().await;
        buffer.insert((data.timestamp, data.nanos), data);
        Ok(())
    }

    pub async fn add_trade_data(&self, data: UnifiedTradeData) -> Result<()> {
        let mut buffer = self.trade_data.lock().await;
        buffer.insert(data.trade_id, data);
        Ok(())
    }

    pub async fn flush_to_disk(&self) -> Result<()> {
        let now = Utc::now();

        // Flush market data
        let market_data = {
            let mut buffer = self.market_data.lock().await;
            std::mem::take(&mut *buffer)
        };

        for (_, data) in market_data {
            let handler = self
                .file_manager
                .get_or_create(data.exchange, &data.symbol, now)
                .await?;

            let mut handler = handler.lock().await;
            handler.write_market_data(&data).await?;
        }

        // Flush trade data
        let trade_data = {
            let mut buffer = self.trade_data.lock().await;
            std::mem::take(&mut *buffer)
        };

        for (_, data) in trade_data {
            let handler = self
                .file_manager
                .get_or_create(data.exchange, &data.symbol, now)
                .await?;

            let mut handler = handler.lock().await;
            handler.write_trade_data(&data).await?;
        }

        // Flush all file handlers
        self.file_manager.flush_all().await?;

        Ok(())
    }

    pub async fn rotate_files_if_needed(&self) -> Result<()> {
        self.file_manager.rotate_if_needed().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_handler_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = FileHandlerManager::new(temp_dir.path().to_path_buf());

        let _handler = manager
            .get_or_create(ExchangeId::Coinbase, "BTC-USD", Utc::now())
            .await
            .unwrap();

        assert!(temp_dir.path().join("coinbase/BTC-USD/MD").exists());
        assert!(temp_dir.path().join("coinbase/BTC-USD/TRADES").exists());
    }

    #[tokio::test]
    async fn test_data_buffer() {
        let temp_dir = TempDir::new().unwrap();
        let buffer = DataBuffer::new(temp_dir.path().to_path_buf());

        let market_data = UnifiedMarketData::new(ExchangeId::Coinbase, "BTC-USD".to_string());
        buffer.add_market_data(market_data).await.unwrap();

        let trade_data = UnifiedTradeData::new(ExchangeId::Binance, "BTC-USDT".to_string(), 12345);
        buffer.add_trade_data(trade_data).await.unwrap();

        buffer.flush_to_disk().await.unwrap();

        // Verify files were created
        assert!(temp_dir.path().join("coinbase/BTC-USD/MD").exists());
        assert!(temp_dir.path().join("binance/BTC-USDT/TRADES").exists());
    }
}
