//! Parser for the server's JSON header + binary data format
//!
//! The server sends data in the following format:
//! 1. JSON header line with column metadata
//! 2. Raw binary data for each column

use crate::buffer_pool::BufferPool;
use gpu_charts_shared::{DataMetadata, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Column metadata from server's JSON header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerColumnMeta {
    pub name: String,
    pub record_size: usize,
    pub num_records: usize,
    pub data_length: usize,
}

/// Server response header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHeader {
    pub columns: Vec<ServerColumnMeta>,
}

/// Set of GPU buffers for a dataset
pub struct ServerGpuBufferSet {
    pub buffers: HashMap<String, Vec<wgpu::Buffer>>,
    pub metadata: DataMetadata,
}

/// Parser for server's JSON + binary format
pub struct ServerParser;

impl ServerParser {
    /// Parse server response format
    pub fn parse_server_response(data: &[u8]) -> Result<(ServerHeader, usize, Vec<u8>)> {
        // Find the newline that separates JSON header from binary data
        let newline_pos = data
            .iter()
            .position(|&b| b == b'\n')
            .ok_or_else(|| Error::ParseError("No newline found in response".to_string()))?;

        // Parse JSON header
        let header_str = std::str::from_utf8(&data[..newline_pos])
            .map_err(|e| Error::ParseError(format!("Invalid UTF-8 in header: {}", e)))?;

        let header: ServerHeader = serde_json::from_str(header_str)
            .map_err(|e| Error::ParseError(format!("Failed to parse JSON header: {}", e)))?;

        // Binary data starts after the newline
        let binary_start = newline_pos + 1;
        let binary_data = data[binary_start..].to_vec();

        Ok((header, binary_start, binary_data))
    }

    /// Parse server data directly to GPU buffers
    pub fn parse_to_gpu_buffers(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        header: &ServerHeader,
        binary_data: &[u8],
        buffer_pool: &mut BufferPool,
    ) -> Result<HashMap<String, Vec<Arc<wgpu::Buffer>>>> {
        let mut buffers = HashMap::new();
        let mut offset = 0;

        // Calculate buffer sizes
        const MAX_BUFFER_SIZE: usize = 128 * 1024 * 1024; // 128MB max per buffer

        for column_meta in &header.columns {
            let total_bytes = column_meta.data_length;

            // Validate we have enough data
            if offset + total_bytes > binary_data.len() {
                return Err(Error::ParseError(format!(
                    "Not enough data for column {}: expected {} bytes at offset {}, but only {} bytes remain",
                    column_meta.name,
                    total_bytes,
                    offset,
                    binary_data.len() - offset
                )));
            }

            // Split into chunks if needed
            let mut column_buffers = Vec::new();
            let mut remaining_bytes = total_bytes;
            let mut data_offset = offset;

            while remaining_bytes > 0 {
                let chunk_size = remaining_bytes.min(MAX_BUFFER_SIZE);

                // Get buffer from pool or create new one
                let buffer = buffer_pool.acquire(device, chunk_size as u64);

                // Write data to buffer
                queue.write_buffer(
                    &buffer,
                    0,
                    &binary_data[data_offset..data_offset + chunk_size],
                );

                column_buffers.push(Arc::new(buffer));

                remaining_bytes -= chunk_size;
                data_offset += chunk_size;
            }

            buffers.insert(column_meta.name.clone(), column_buffers);
            offset += total_bytes;
        }

        Ok(buffers)
    }

    /// Convert server header to our internal format
    pub fn get_row_count(header: &ServerHeader) -> u32 {
        // All columns should have the same number of records
        header
            .columns
            .first()
            .map(|col| col.num_records as u32)
            .unwrap_or(0)
    }

    /// Get column names from server header
    pub fn get_column_names(header: &ServerHeader) -> Vec<String> {
        header.columns.iter().map(|col| col.name.clone()).collect()
    }

    /// Calculate total data size
    pub fn get_total_size(header: &ServerHeader) -> u64 {
        header
            .columns
            .iter()
            .map(|col| col.data_length as u64)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_server_response() {
        let test_data = br#"{"columns":[{"name":"time","record_size":4,"num_records":10,"data_length":40},{"name":"price","record_size":4,"num_records":10,"data_length":40}]}
BINARY_DATA_HERE"#;

        let (header, header_size, binary_data) =
            ServerParser::parse_server_response(test_data).unwrap();

        assert_eq!(header.columns.len(), 2);
        assert_eq!(header.columns[0].name, "time");
        assert_eq!(header.columns[0].num_records, 10);
        assert_eq!(header.columns[1].name, "price");
        assert_eq!(binary_data, b"BINARY_DATA_HERE");
        assert_eq!(header_size, 128); // Length of JSON + newline
    }

    #[test]
    fn test_get_row_count() {
        let header = ServerHeader {
            columns: vec![ServerColumnMeta {
                name: "time".to_string(),
                record_size: 4,
                num_records: 100,
                data_length: 400,
            }],
        };

        assert_eq!(ServerParser::get_row_count(&header), 100);
    }
}
