//! Chunked parsing system for handling datasets larger than memory
//!
//! This module implements streaming parsers that can handle massive datasets
//! by processing them in chunks, with backpressure and memory management.

use crate::simd::{ColumnData, ProcessedColumn, SimdBatchProcessor};
use futures::stream::Stream;
use gpu_charts_shared::{Error, Result};
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader};

/// Configuration for chunked parsing
#[derive(Debug, Clone)]
pub struct ChunkedParserConfig {
    /// Maximum chunk size in bytes
    pub chunk_size: usize,
    /// Maximum memory usage in bytes
    pub max_memory: usize,
    /// Enable SIMD optimizations
    pub use_simd: bool,
    /// Number of chunks to buffer ahead
    pub lookahead_chunks: usize,
}

impl Default for ChunkedParserConfig {
    fn default() -> Self {
        Self {
            chunk_size: 64 * 1024 * 1024,  // 64MB chunks
            max_memory: 512 * 1024 * 1024, // 512MB max
            use_simd: true,
            lookahead_chunks: 2,
        }
    }
}

/// A chunk of parsed data ready for GPU upload
pub struct DataChunk {
    /// Chunk index
    pub index: usize,
    /// Start offset in the original data
    pub offset: u64,
    /// Parsed column data
    pub columns: Vec<ChunkColumn>,
    /// Number of rows in this chunk
    pub row_count: u32,
    /// Memory usage in bytes
    pub memory_usage: usize,
}

/// Column data within a chunk
pub enum ChunkColumn {
    F32(Vec<f32>),
    F64(Vec<f64>),
    U64(Vec<u64>),
    I64(Vec<i64>),
}

impl ChunkColumn {
    /// Get memory usage of this column
    pub fn memory_usage(&self) -> usize {
        match self {
            ChunkColumn::F32(v) => v.len() * 4,
            ChunkColumn::F64(v) => v.len() * 8,
            ChunkColumn::U64(v) => v.len() * 8,
            ChunkColumn::I64(v) => v.len() * 8,
        }
    }
}

/// Streaming parser for large datasets
pub struct ChunkedParser {
    config: ChunkedParserConfig,
    simd_processor: Option<SimdBatchProcessor>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
}

impl ChunkedParser {
    /// Create a new chunked parser
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        config: ChunkedParserConfig,
    ) -> Self {
        let simd_processor = if config.use_simd {
            Some(SimdBatchProcessor::new())
        } else {
            None
        };

        Self {
            config,
            simd_processor,
            device,
            queue,
        }
    }

    /// Parse a file in chunks
    pub async fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        column_types: Vec<ColumnType>,
    ) -> Result<ChunkedDataStream> {
        let file = File::open(path.as_ref())
            .await
            .map_err(|e| Error::ParseError(e.to_string()))?;

        let metadata = file
            .metadata()
            .await
            .map_err(|e| Error::ParseError(e.to_string()))?;

        let file_size = metadata.len();

        Ok(ChunkedDataStream::new(
            Box::pin(file),
            file_size,
            column_types,
            self.config.clone(),
            self.simd_processor.as_ref(),
        ))
    }

    /// Parse from any async reader
    pub fn parse_stream<R: AsyncRead + Unpin + Send + 'static>(
        &self,
        reader: R,
        estimated_size: u64,
        column_types: Vec<ColumnType>,
    ) -> ChunkedDataStream {
        // TODO: Fix lifetime issue - for now pass None
        // The SIMD processor reference needs 'static lifetime but self only has local lifetime
        ChunkedDataStream::new(
            Box::pin(reader),
            estimated_size,
            column_types,
            self.config.clone(),
            None, // TODO: self.simd_processor.as_ref() needs 'static lifetime
        )
    }

    /// Create GPU buffers from a chunk
    pub fn chunk_to_gpu_buffers(&self, chunk: DataChunk) -> Result<ChunkGpuBuffers> {
        let mut buffers = Vec::new();

        for column in chunk.columns {
            let buffer = match column {
                ChunkColumn::F32(data) => self.create_buffer_from_slice(&data, "f32_column")?,
                ChunkColumn::F64(data) => self.create_buffer_from_slice(&data, "f64_column")?,
                ChunkColumn::U64(data) => self.create_buffer_from_slice(&data, "u64_column")?,
                ChunkColumn::I64(data) => self.create_buffer_from_slice(&data, "i64_column")?,
            };
            buffers.push(buffer);
        }

        Ok(ChunkGpuBuffers {
            chunk_index: chunk.index,
            offset: chunk.offset,
            row_count: chunk.row_count,
            buffers,
        })
    }

    fn create_buffer_from_slice<T: bytemuck::Pod>(
        &self,
        data: &[T],
        label: &str,
    ) -> Result<wgpu::Buffer> {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (data.len() * std::mem::size_of::<T>()) as u64,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.queue
            .write_buffer(&buffer, 0, bytemuck::cast_slice(data));

        Ok(buffer)
    }
}

/// Column type information
#[derive(Debug, Clone, Copy)]
pub enum ColumnType {
    F32,
    F64,
    U64,
    I64,
}

/// Stream of data chunks
pub struct ChunkedDataStream {
    reader: Pin<Box<dyn AsyncRead + Send>>,
    file_size: u64,
    column_types: Vec<ColumnType>,
    config: ChunkedParserConfig,
    simd_processor: Option<&'static SimdBatchProcessor>,
    current_offset: u64,
    chunk_index: usize,
    buffer: Vec<u8>,
    memory_pressure: MemoryPressureTracker,
}

impl ChunkedDataStream {
    fn new(
        reader: Pin<Box<dyn AsyncRead + Send>>,
        file_size: u64,
        column_types: Vec<ColumnType>,
        config: ChunkedParserConfig,
        simd_processor: Option<&'static SimdBatchProcessor>,
    ) -> Self {
        let buffer = Vec::with_capacity(config.chunk_size);
        let max_memory = config.max_memory;

        Self {
            reader,
            file_size,
            column_types,
            config,
            simd_processor,
            current_offset: 0,
            chunk_index: 0,
            buffer,
            memory_pressure: MemoryPressureTracker::new(max_memory),
        }
    }

    /// Read and parse the next chunk
    async fn read_next_chunk(&mut self) -> Result<Option<DataChunk>> {
        // Check memory pressure
        if self.memory_pressure.should_wait() {
            // Apply backpressure
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Clear and resize buffer
        self.buffer.clear();
        self.buffer.resize(self.config.chunk_size, 0);

        // Read chunk
        let bytes_read = self
            .reader
            .read(&mut self.buffer)
            .await
            .map_err(|e| Error::ParseError(e.to_string()))?;

        if bytes_read == 0 {
            return Ok(None);
        }

        // Resize buffer to actual read size
        self.buffer.truncate(bytes_read);

        // Parse chunk
        let chunk = self.parse_chunk()?;

        // Update tracking
        self.current_offset += bytes_read as u64;
        self.chunk_index += 1;
        self.memory_pressure.add_memory(chunk.memory_usage);

        Ok(Some(chunk))
    }

    /// Parse a chunk of binary data
    fn parse_chunk(&self) -> Result<DataChunk> {
        let row_size = self.calculate_row_size();
        let row_count = self.buffer.len() / row_size;

        let mut columns = Vec::new();
        let mut offset = 0;

        for &col_type in &self.column_types {
            let column_data = match col_type {
                ColumnType::F32 => {
                    let mut data = Vec::with_capacity(row_count);
                    for i in 0..row_count {
                        let row_offset = i * row_size + offset;
                        let bytes = &self.buffer[row_offset..row_offset + 4];
                        let value = f32::from_le_bytes(bytes.try_into().unwrap());
                        data.push(value);
                    }
                    offset += 4;
                    ChunkColumn::F32(data)
                }
                ColumnType::F64 => {
                    let mut data = Vec::with_capacity(row_count);
                    for i in 0..row_count {
                        let row_offset = i * row_size + offset;
                        let bytes = &self.buffer[row_offset..row_offset + 8];
                        let value = f64::from_le_bytes(bytes.try_into().unwrap());
                        data.push(value);
                    }
                    offset += 8;
                    ChunkColumn::F64(data)
                }
                ColumnType::U64 => {
                    let mut data = Vec::with_capacity(row_count);
                    for i in 0..row_count {
                        let row_offset = i * row_size + offset;
                        let bytes = &self.buffer[row_offset..row_offset + 8];
                        let value = u64::from_le_bytes(bytes.try_into().unwrap());
                        data.push(value);
                    }
                    offset += 8;
                    ChunkColumn::U64(data)
                }
                ColumnType::I64 => {
                    let mut data = Vec::with_capacity(row_count);
                    for i in 0..row_count {
                        let row_offset = i * row_size + offset;
                        let bytes = &self.buffer[row_offset..row_offset + 8];
                        let value = i64::from_le_bytes(bytes.try_into().unwrap());
                        data.push(value);
                    }
                    offset += 8;
                    ChunkColumn::I64(data)
                }
            };
            columns.push(column_data);
        }

        // Apply SIMD processing if available
        if let Some(processor) = self.simd_processor {
            columns = self.apply_simd_processing(columns, processor);
        }

        let memory_usage = columns.iter().map(|c| c.memory_usage()).sum();

        Ok(DataChunk {
            index: self.chunk_index,
            offset: self.current_offset,
            columns,
            row_count: row_count as u32,
            memory_usage,
        })
    }

    fn calculate_row_size(&self) -> usize {
        self.column_types
            .iter()
            .map(|&t| match t {
                ColumnType::F32 => 4,
                ColumnType::F64 => 8,
                ColumnType::U64 => 8,
                ColumnType::I64 => 8,
            })
            .sum()
    }

    fn apply_simd_processing(
        &self,
        columns: Vec<ChunkColumn>,
        _processor: &SimdBatchProcessor,
    ) -> Vec<ChunkColumn> {
        // Convert to SIMD format, process, and convert back
        // This is a simplified version - real implementation would be more sophisticated
        columns
    }
}

impl Stream for ChunkedDataStream {
    type Item = Result<DataChunk>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Use tokio runtime to execute async operation
        let future = self.read_next_chunk();
        let mut pinned = Box::pin(future);

        match pinned.as_mut().poll(cx) {
            Poll::Ready(Ok(Some(chunk))) => Poll::Ready(Some(Ok(chunk))),
            Poll::Ready(Ok(None)) => Poll::Ready(None),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// GPU buffers for a chunk
pub struct ChunkGpuBuffers {
    pub chunk_index: usize,
    pub offset: u64,
    pub row_count: u32,
    pub buffers: Vec<wgpu::Buffer>,
}

/// Track memory pressure for backpressure
struct MemoryPressureTracker {
    current_usage: usize,
    max_memory: usize,
    high_water_mark: f64,
}

impl MemoryPressureTracker {
    fn new(max_memory: usize) -> Self {
        Self {
            current_usage: 0,
            max_memory,
            high_water_mark: 0.8, // Start backpressure at 80%
        }
    }

    fn add_memory(&mut self, bytes: usize) {
        self.current_usage += bytes;
    }

    fn remove_memory(&mut self, bytes: usize) {
        self.current_usage = self.current_usage.saturating_sub(bytes);
    }

    fn should_wait(&self) -> bool {
        let usage_ratio = self.current_usage as f64 / self.max_memory as f64;
        usage_ratio > self.high_water_mark
    }

    fn memory_available(&self) -> usize {
        self.max_memory.saturating_sub(self.current_usage)
    }
}

/// Chunk coordinator for managing multiple chunks
pub struct ChunkCoordinator {
    chunks: Vec<ChunkGpuBuffers>,
    total_rows: u64,
    memory_tracker: MemoryPressureTracker,
}

impl ChunkCoordinator {
    pub fn new(max_memory: usize) -> Self {
        Self {
            chunks: Vec::new(),
            total_rows: 0,
            memory_tracker: MemoryPressureTracker::new(max_memory),
        }
    }

    pub fn add_chunk(&mut self, chunk: ChunkGpuBuffers) {
        self.total_rows += chunk.row_count as u64;
        self.chunks.push(chunk);
    }

    pub fn get_visible_chunks(&self, start_row: u64, end_row: u64) -> Vec<usize> {
        let mut visible = Vec::new();
        let mut current_row = 0u64;

        for (i, chunk) in self.chunks.iter().enumerate() {
            let chunk_end = current_row + chunk.row_count as u64;

            if current_row <= end_row && chunk_end >= start_row {
                visible.push(i);
            }

            current_row = chunk_end;
        }

        visible
    }

    pub fn release_chunk(&mut self, index: usize) {
        if index < self.chunks.len() {
            self.chunks.remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_memory_usage() {
        let f32_col = ChunkColumn::F32(vec![1.0; 1000]);
        assert_eq!(f32_col.memory_usage(), 4000);

        let u64_col = ChunkColumn::U64(vec![1; 1000]);
        assert_eq!(u64_col.memory_usage(), 8000);
    }

    #[test]
    fn test_memory_pressure_tracker() {
        let mut tracker = MemoryPressureTracker::new(1000);

        tracker.add_memory(700);
        assert!(!tracker.should_wait());

        tracker.add_memory(200);
        assert!(tracker.should_wait()); // 90% > 80%

        tracker.remove_memory(300);
        assert!(!tracker.should_wait()); // 60% < 80%
    }

    #[test]
    fn test_chunk_coordinator() {
        let mut coordinator = ChunkCoordinator::new(1024 * 1024);

        let chunk1 = ChunkGpuBuffers {
            chunk_index: 0,
            offset: 0,
            row_count: 1000,
            buffers: vec![],
        };

        let chunk2 = ChunkGpuBuffers {
            chunk_index: 1,
            offset: 1000,
            row_count: 1000,
            buffers: vec![],
        };

        coordinator.add_chunk(chunk1);
        coordinator.add_chunk(chunk2);

        // Test visible chunks
        let visible = coordinator.get_visible_chunks(500, 1500);
        assert_eq!(visible, vec![0, 1]);

        let visible = coordinator.get_visible_chunks(0, 500);
        assert_eq!(visible, vec![0]);

        let visible = coordinator.get_visible_chunks(1500, 2000);
        assert_eq!(visible, vec![1]);
    }
}
