use futures_util::future::try_join_all;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::Result;

pub const FILE_BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer for file writes

pub struct FileHandles {
    pub time_file: BufWriter<File>,
    pub nanos_file: BufWriter<File>,
    pub price_file: BufWriter<File>,
    pub volume_file: BufWriter<File>,
    pub side_file: BufWriter<File>,
    pub best_bid_file: BufWriter<File>,
    pub best_ask_file: BufWriter<File>,
}

/// Trade-specific file handles for enhanced trade logging
pub struct TradeFileHandles {
    pub trade_time_file: BufWriter<File>,
    pub trade_nanos_file: BufWriter<File>,
    pub trade_price_file: BufWriter<File>,
    pub trade_volume_file: BufWriter<File>,
    pub trade_side_file: BufWriter<File>,
    pub trade_spread_file: BufWriter<File>,
}

/// Market trades file handles for individual trade data
pub struct MarketTradeFileHandles {
    pub trade_id_file: BufWriter<File>,
    pub trade_time_file: BufWriter<File>,
    pub trade_nanos_file: BufWriter<File>,
    pub trade_price_file: BufWriter<File>,
    pub trade_size_file: BufWriter<File>,
    pub trade_side_file: BufWriter<File>,
    pub maker_order_id_file: BufWriter<File>,
    pub taker_order_id_file: BufWriter<File>,
}

impl FileHandles {
    pub async fn flush_all(&mut self) -> Result<()> {
        // Flush all files in parallel
        let flush_futures = vec![
            self.time_file.flush(),
            self.nanos_file.flush(),
            self.price_file.flush(),
            self.volume_file.flush(),
            self.side_file.flush(),
            self.best_bid_file.flush(),
            self.best_ask_file.flush(),
        ];

        try_join_all(flush_futures).await?;
        Ok(())
    }

    pub async fn close(mut self) -> Result<()> {
        // Flush all buffers before closing
        self.flush_all().await?;

        // Shutdown all writers to ensure data is written
        self.time_file.shutdown().await?;
        self.nanos_file.shutdown().await?;
        self.price_file.shutdown().await?;
        self.volume_file.shutdown().await?;
        self.side_file.shutdown().await?;
        self.best_bid_file.shutdown().await?;
        self.best_ask_file.shutdown().await?;

        Ok(())
    }
}

impl TradeFileHandles {
    pub async fn flush_all(&mut self) -> Result<()> {
        // Flush all trade files in parallel
        let flush_futures = vec![
            self.trade_time_file.flush(),
            self.trade_nanos_file.flush(),
            self.trade_price_file.flush(),
            self.trade_volume_file.flush(),
            self.trade_side_file.flush(),
            self.trade_spread_file.flush(),
        ];

        try_join_all(flush_futures).await?;
        Ok(())
    }

    pub async fn close(mut self) -> Result<()> {
        // Flush all buffers before closing
        self.flush_all().await?;

        // Shutdown all writers to ensure data is written
        self.trade_time_file.shutdown().await?;
        self.trade_nanos_file.shutdown().await?;
        self.trade_price_file.shutdown().await?;
        self.trade_volume_file.shutdown().await?;
        self.trade_side_file.shutdown().await?;
        self.trade_spread_file.shutdown().await?;

        Ok(())
    }
}

impl MarketTradeFileHandles {
    pub async fn flush_all(&mut self) -> Result<()> {
        // Flush all market trade files in parallel
        let flush_futures = vec![
            self.trade_id_file.flush(),
            self.trade_time_file.flush(),
            self.trade_nanos_file.flush(),
            self.trade_price_file.flush(),
            self.trade_size_file.flush(),
            self.trade_side_file.flush(),
            self.maker_order_id_file.flush(),
            self.taker_order_id_file.flush(),
        ];

        try_join_all(flush_futures).await?;
        Ok(())
    }

    pub async fn close(mut self) -> Result<()> {
        // Flush all buffers before closing
        self.flush_all().await?;

        // Shutdown all writers to ensure data is written
        self.trade_id_file.shutdown().await?;
        self.trade_time_file.shutdown().await?;
        self.trade_nanos_file.shutdown().await?;
        self.trade_price_file.shutdown().await?;
        self.trade_size_file.shutdown().await?;
        self.trade_side_file.shutdown().await?;
        self.maker_order_id_file.shutdown().await?;
        self.taker_order_id_file.shutdown().await?;

        Ok(())
    }
}

/// Analytics file handles for aggregated data
pub struct AnalyticsFileHandles {
    // Candle files (per period)
    pub candle_files: std::collections::HashMap<u32, CandleFileHandles>,
    // Metrics files
    pub trade_intensity_file: BufWriter<File>,
    pub momentum_file: BufWriter<File>,
    pub buy_sell_ratio_file: BufWriter<File>,
    // Significant trades
    pub significant_trade_ids_file: BufWriter<File>,
    pub significant_scores_file: BufWriter<File>,
    pub significant_impacts_file: BufWriter<File>,
}

pub struct CandleFileHandles {
    pub ohlc_file: BufWriter<File>,
    pub volume_file: BufWriter<File>,
    pub vwap_file: BufWriter<File>,
}

impl CandleFileHandles {
    pub async fn flush_all(&mut self) -> Result<()> {
        let flush_futures = vec![
            self.ohlc_file.flush(),
            self.volume_file.flush(),
            self.vwap_file.flush(),
        ];
        try_join_all(flush_futures).await?;
        Ok(())
    }

    pub async fn close(mut self) -> Result<()> {
        self.flush_all().await?;
        self.ohlc_file.shutdown().await?;
        self.volume_file.shutdown().await?;
        self.vwap_file.shutdown().await?;
        Ok(())
    }
}

impl AnalyticsFileHandles {
    pub async fn flush_all(&mut self) -> Result<()> {
        // Flush candle files
        for (_, handles) in self.candle_files.iter_mut() {
            handles.flush_all().await?;
        }

        // Flush metrics files
        let flush_futures = vec![
            self.trade_intensity_file.flush(),
            self.momentum_file.flush(),
            self.buy_sell_ratio_file.flush(),
            self.significant_trade_ids_file.flush(),
            self.significant_scores_file.flush(),
            self.significant_impacts_file.flush(),
        ];
        try_join_all(flush_futures).await?;
        Ok(())
    }

    pub async fn close(mut self) -> Result<()> {
        self.flush_all().await?;

        // Close candle files
        let candle_files: Vec<_> = self.candle_files.into_iter().collect();
        for (_, handles) in candle_files {
            handles.close().await?;
        }

        // Close metrics files
        self.trade_intensity_file.shutdown().await?;
        self.momentum_file.shutdown().await?;
        self.buy_sell_ratio_file.shutdown().await?;
        self.significant_trade_ids_file.shutdown().await?;
        self.significant_scores_file.shutdown().await?;
        self.significant_impacts_file.shutdown().await?;
        Ok(())
    }
}

pub async fn open_file(path: &str) -> Result<File> {
    Ok(tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?)
}
