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

pub async fn open_file(path: &str) -> Result<File> {
    Ok(tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?)
}