//! Compression support for efficient data transfer
//!
//! This module implements transparent compression/decompression with support
//! for Gzip, Brotli, and Zstandard compression algorithms.

use gpu_charts_shared::{Error, Result};
use std::io::{Read, Write};

/// Supported compression algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    None,
    Gzip,
    Brotli,
    Zstd,
}

impl CompressionType {
    /// Detect compression type from content-encoding header
    pub fn from_content_encoding(encoding: &str) -> Self {
        match encoding.to_lowercase().as_str() {
            "gzip" => Self::Gzip,
            "br" => Self::Brotli,
            "zstd" => Self::Zstd,
            _ => Self::None,
        }
    }

    /// Get content-encoding header value
    pub fn to_content_encoding(&self) -> &'static str {
        match self {
            Self::None => "identity",
            Self::Gzip => "gzip",
            Self::Brotli => "br",
            Self::Zstd => "zstd",
        }
    }

    /// Get file extension for compressed files
    pub fn extension(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Gzip => ".gz",
            Self::Brotli => ".br",
            Self::Zstd => ".zst",
        }
    }
}

/// Compression configuration
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Preferred compression type
    pub preferred_type: CompressionType,
    /// Compression level (1-9, higher = better compression)
    pub level: u32,
    /// Minimum size to compress (bytes)
    pub min_size: usize,
    /// Enable auto-detection of compressed data
    pub auto_detect: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            preferred_type: CompressionType::Gzip,
            level: 6,
            min_size: 1024, // 1KB
            auto_detect: true,
        }
    }
}

/// Compression/decompression utility
pub struct CompressionHandler {
    config: CompressionConfig,
}

impl CompressionHandler {
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    /// Compress data using configured algorithm
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < self.config.min_size {
            return Ok(data.to_vec());
        }

        match self.config.preferred_type {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => self.compress_gzip(data),
            CompressionType::Brotli => self.compress_brotli(data),
            CompressionType::Zstd => self.compress_zstd(data),
        }
    }

    /// Decompress data with auto-detection
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        // Auto-detect compression type
        let compression_type = if self.config.auto_detect {
            self.detect_compression(data)
        } else {
            CompressionType::None
        };

        match compression_type {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => self.decompress_gzip(data),
            CompressionType::Brotli => self.decompress_brotli(data),
            CompressionType::Zstd => self.decompress_zstd(data),
        }
    }

    /// Detect compression type from data
    fn detect_compression(&self, data: &[u8]) -> CompressionType {
        if data.len() < 2 {
            return CompressionType::None;
        }

        // Check magic bytes
        match (data[0], data[1]) {
            // Gzip magic: 0x1f 0x8b
            (0x1f, 0x8b) => CompressionType::Gzip,
            // Zstd magic: 0x28 0xb5
            (0x28, 0xb5) => CompressionType::Zstd,
            // Brotli doesn't have consistent magic bytes
            _ => {
                // Try Brotli decompression as fallback
                if self.is_brotli(data) {
                    CompressionType::Brotli
                } else {
                    CompressionType::None
                }
            }
        }
    }

    /// Check if data might be Brotli compressed
    fn is_brotli(&self, data: &[u8]) -> bool {
        // Simple heuristic: try decompressing first few bytes
        if data.len() < 10 {
            return false;
        }

        let mut decoder = brotli::Decompressor::new(&data[..10], 4096);
        let mut output = vec![0u8; 10];
        decoder.read(&mut output).is_ok()
    }

    /// Compress using Gzip
    fn compress_gzip(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let level = match self.config.level {
            1..=3 => Compression::fast(),
            4..=6 => Compression::default(),
            7..=9 => Compression::best(),
            _ => Compression::default(),
        };

        let mut encoder = GzEncoder::new(Vec::new(), level);
        encoder
            .write_all(data)
            .map_err(|e| Error::ParseError(format!("Gzip compression failed: {}", e)))?;

        encoder
            .finish()
            .map_err(|e| Error::ParseError(format!("Gzip compression failed: {}", e)))
    }

    /// Decompress Gzip data
    fn decompress_gzip(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::read::GzDecoder;

        let mut decoder = GzDecoder::new(data);
        let mut output = Vec::new();

        decoder
            .read_to_end(&mut output)
            .map_err(|e| Error::ParseError(format!("Gzip decompression failed: {}", e)))?;

        Ok(output)
    }

    /// Compress using Brotli
    fn compress_brotli(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        let mut compressor = brotli::CompressorWriter::new(
            &mut output,
            4096,
            self.config.level,
            22, // lgwin
        );

        compressor
            .write_all(data)
            .map_err(|e| Error::ParseError(format!("Brotli compression failed: {}", e)))?;

        drop(compressor);
        Ok(output)
    }

    /// Decompress Brotli data
    fn decompress_brotli(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut decoder = brotli::Decompressor::new(data, 4096);
        let mut output = Vec::new();

        decoder
            .read_to_end(&mut output)
            .map_err(|e| Error::ParseError(format!("Brotli decompression failed: {}", e)))?;

        Ok(output)
    }

    /// Compress using Zstandard
    fn compress_zstd(&self, data: &[u8]) -> Result<Vec<u8>> {
        #[cfg(feature = "native")]
        {
            zstd::encode_all(data, self.config.level as i32)
                .map_err(|e| Error::ParseError(format!("Zstd compression failed: {}", e)))
        }
        #[cfg(not(feature = "native"))]
        {
            // Zstd not available in WASM, fall back to gzip
            self.compress_gzip(data)
        }
    }

    /// Decompress Zstandard data
    fn decompress_zstd(&self, data: &[u8]) -> Result<Vec<u8>> {
        #[cfg(feature = "native")]
        {
            zstd::decode_all(data)
                .map_err(|e| Error::ParseError(format!("Zstd decompression failed: {}", e)))
        }
        #[cfg(not(feature = "native"))]
        {
            // Zstd not available in WASM, fall back to gzip
            self.decompress_gzip(data)
        }
    }

    /// Get compression statistics
    pub fn get_compression_ratio(&self, original: &[u8], compressed: &[u8]) -> f32 {
        if original.is_empty() {
            return 1.0;
        }

        compressed.len() as f32 / original.len() as f32
    }
}

/// Stream compression for large data
pub struct StreamCompressor {
    compression_type: CompressionType,
    level: u32,
}

impl StreamCompressor {
    pub fn new(compression_type: CompressionType, level: u32) -> Self {
        Self {
            compression_type,
            level,
        }
    }

    /// Create a compressing writer
    pub fn create_writer<W: Write + 'static>(&self, writer: W) -> Box<dyn Write> {
        match self.compression_type {
            CompressionType::None => Box::new(writer),
            CompressionType::Gzip => {
                use flate2::write::GzEncoder;
                use flate2::Compression;

                let level = match self.level {
                    1..=3 => Compression::fast(),
                    4..=6 => Compression::default(),
                    7..=9 => Compression::best(),
                    _ => Compression::default(),
                };

                Box::new(GzEncoder::new(writer, level))
            }
            CompressionType::Brotli => {
                Box::new(brotli::CompressorWriter::new(writer, 4096, self.level, 22))
            }
            CompressionType::Zstd => {
                #[cfg(feature = "native")]
                {
                    Box::new(zstd::Encoder::new(writer, self.level as i32).unwrap())
                }
                #[cfg(not(feature = "native"))]
                {
                    // Zstd not available in WASM, use gzip instead
                    use flate2::write::GzEncoder;
                    Box::new(GzEncoder::new(
                        writer,
                        flate2::Compression::new(self.level as u32),
                    ))
                }
            }
        }
    }

    /// Create a decompressing reader
    pub fn create_reader<R: Read + 'static>(&self, reader: R) -> Box<dyn Read> {
        match self.compression_type {
            CompressionType::None => Box::new(reader),
            CompressionType::Gzip => {
                use flate2::read::GzDecoder;
                Box::new(GzDecoder::new(reader))
            }
            CompressionType::Brotli => Box::new(brotli::Decompressor::new(reader, 4096)),
            CompressionType::Zstd => {
                #[cfg(feature = "native")]
                {
                    Box::new(zstd::Decoder::new(reader).unwrap())
                }
                #[cfg(not(feature = "native"))]
                {
                    // Zstd not available in WASM, use gzip instead
                    use flate2::read::GzDecoder;
                    Box::new(GzDecoder::new(reader))
                }
            }
        }
    }
}

/// Compression benchmark results
#[derive(Debug)]
pub struct CompressionBenchmark {
    pub algorithm: CompressionType,
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_time_ms: f32,
    pub decompression_time_ms: f32,
    pub ratio: f32,
}

impl CompressionBenchmark {
    /// Run benchmark on data
    pub fn benchmark(data: &[u8], compression_type: CompressionType, level: u32) -> Self {
        let config = CompressionConfig {
            preferred_type: compression_type,
            level,
            min_size: 0,
            auto_detect: false,
        };

        let handler = CompressionHandler::new(config);

        // Measure compression
        let compress_start = std::time::Instant::now();
        let compressed = handler.compress(data).unwrap_or_default();
        let compression_time_ms = compress_start.elapsed().as_secs_f32() * 1000.0;

        // Measure decompression
        let decompress_start = std::time::Instant::now();
        let _ = handler.decompress(&compressed);
        let decompression_time_ms = decompress_start.elapsed().as_secs_f32() * 1000.0;

        Self {
            algorithm: compression_type,
            original_size: data.len(),
            compressed_size: compressed.len(),
            compression_time_ms,
            decompression_time_ms,
            ratio: handler.get_compression_ratio(data, &compressed),
        }
    }

    /// Compare different algorithms
    pub fn compare_algorithms(data: &[u8], level: u32) -> Vec<Self> {
        vec![
            Self::benchmark(data, CompressionType::Gzip, level),
            Self::benchmark(data, CompressionType::Brotli, level),
            Self::benchmark(data, CompressionType::Zstd, level),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_detection() {
        let handler = CompressionHandler::new(CompressionConfig::default());

        // Test Gzip detection
        let gzip_data = vec![0x1f, 0x8b, 0x08, 0x00];
        assert_eq!(
            handler.detect_compression(&gzip_data),
            CompressionType::Gzip
        );

        // Test Zstd detection
        let zstd_data = vec![0x28, 0xb5, 0x2f, 0xfd];
        assert_eq!(
            handler.detect_compression(&zstd_data),
            CompressionType::Zstd
        );
    }

    #[test]
    fn test_round_trip() {
        let handler = CompressionHandler::new(CompressionConfig::default());
        let data = b"Hello, world! This is a test of compression.";

        let compressed = handler.compress(data).unwrap();
        let decompressed = handler.decompress(&compressed).unwrap();

        assert_eq!(data, &decompressed[..]);
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_compression_ratio() {
        let handler = CompressionHandler::new(CompressionConfig::default());

        let original = vec![b'A'; 1000];
        let compressed = handler.compress(&original).unwrap();

        let ratio = handler.get_compression_ratio(&original, &compressed);
        assert!(ratio < 0.1); // Highly compressible data
    }
}
