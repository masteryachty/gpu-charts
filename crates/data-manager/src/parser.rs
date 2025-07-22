//! Zero-copy binary parser for direct GPU buffer creation

use crate::buffer_pool::BufferPool;
use gpu_charts_shared::{DataMetadata, Error, Result};
use std::collections::HashMap;

/// Header structure for binary data format
#[derive(Debug, Clone)]
pub struct BinaryHeader {
    pub columns: Vec<String>,
    pub row_count: u32,
    pub column_types: Vec<ColumnType>,
}

/// Set of GPU buffers for a dataset
pub struct GpuBufferSet {
    pub buffers: HashMap<String, Vec<wgpu::Buffer>>,
    pub metadata: DataMetadata,
}

#[derive(Debug, Clone, Copy)]
pub enum ColumnType {
    U32,
    F32,
    U64,
    F64,
}

impl ColumnType {
    pub fn byte_size(&self) -> usize {
        match self {
            ColumnType::U32 | ColumnType::F32 => 4,
            ColumnType::U64 | ColumnType::F64 => 8,
        }
    }
}

/// Zero-copy parser that creates GPU buffers directly from binary data
pub struct BinaryParser;

impl BinaryParser {
    /// Parse binary data header
    pub fn parse_header(data: &[u8]) -> Result<(BinaryHeader, usize)> {
        if data.len() < 16 {
            return Err(Error::ParseError("Data too small for header".to_string()));
        }

        // Read header format:
        // [0..4] magic number (0x47504348 = "GPCH")
        // [4..8] header size (u32)
        // [8..12] row count (u32)
        // [12..16] column count (u32)
        // Then column definitions...

        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != 0x47504348 {
            return Err(Error::ParseError("Invalid magic number".to_string()));
        }

        let header_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let row_count = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let column_count = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;

        // Parse column definitions
        let mut offset = 16;
        let mut columns = Vec::with_capacity(column_count);
        let mut column_types = Vec::with_capacity(column_count);

        for _ in 0..column_count {
            if offset + 5 > data.len() {
                return Err(Error::ParseError("Invalid column definition".to_string()));
            }

            // Read column name length
            let name_len = data[offset] as usize;
            offset += 1;

            if offset + name_len + 1 > data.len() {
                return Err(Error::ParseError("Invalid column name".to_string()));
            }

            // Read column name
            let name = String::from_utf8(data[offset..offset + name_len].to_vec())
                .map_err(|_| Error::ParseError("Invalid column name encoding".to_string()))?;
            offset += name_len;

            // Read column type
            let col_type = match data[offset] {
                0 => ColumnType::U32,
                1 => ColumnType::F32,
                2 => ColumnType::U64,
                3 => ColumnType::F64,
                _ => return Err(Error::ParseError("Invalid column type".to_string())),
            };
            offset += 1;

            columns.push(name);
            column_types.push(col_type);
        }

        let header = BinaryHeader {
            columns,
            row_count,
            column_types,
        };

        Ok((header, header_size))
    }

    /// Parse binary data directly to GPU buffers without intermediate allocations
    pub fn parse_to_gpu_buffers(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        header: &BinaryHeader,
        header_size: usize,
        buffer_pool: &mut BufferPool,
    ) -> Result<HashMap<String, Vec<wgpu::Buffer>>> {
        let mut buffers = HashMap::new();
        let mut offset = header_size;

        // Calculate buffer sizes
        const MAX_BUFFER_SIZE: usize = 128 * 1024 * 1024; // 128MB max per buffer

        for (i, column) in header.columns.iter().enumerate() {
            let column_type = header.column_types[i];
            let bytes_per_element = column_type.byte_size();
            let total_bytes = header.row_count as usize * bytes_per_element;

            // Split into chunks if needed
            let mut column_buffers = Vec::new();
            let mut remaining_bytes = total_bytes;
            let mut data_offset = offset;

            while remaining_bytes > 0 {
                let chunk_size = remaining_bytes.min(MAX_BUFFER_SIZE);
                let _chunk_elements = chunk_size / bytes_per_element;

                // Get buffer from pool or create new one
                let buffer = buffer_pool.acquire(device, chunk_size as u64);

                // Write data to buffer
                queue.write_buffer(&buffer, 0, &data[data_offset..data_offset + chunk_size]);

                column_buffers.push(buffer);

                remaining_bytes -= chunk_size;
                data_offset += chunk_size;
            }

            buffers.insert(column.clone(), column_buffers);
            offset += total_bytes;
        }

        Ok(buffers)
    }

    /// Validate data without copying
    pub fn validate_data(data: &[u8], header: &BinaryHeader, header_size: usize) -> Result<()> {
        let expected_size = header_size
            + header
                .columns
                .iter()
                .zip(&header.column_types)
                .map(|(_, typ)| header.row_count as usize * typ.byte_size())
                .sum::<usize>();

        if data.len() < expected_size {
            return Err(Error::ParseError(format!(
                "Data size mismatch: expected {} bytes, got {}",
                expected_size,
                data.len()
            )));
        }

        Ok(())
    }
}

/// SIMD-optimized data transformations
pub mod simd {
    use super::*;

    /// Convert and validate f64 to f32 with SIMD
    pub fn convert_f64_to_f32(input: &[f64], output: &mut [f32]) -> Result<()> {
        if input.len() != output.len() {
            return Err(Error::ParseError("Length mismatch".to_string()));
        }

        // TODO: Use SIMD instructions when available
        for (i, &val) in input.iter().enumerate() {
            output[i] = val as f32;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_type_sizes() {
        assert_eq!(ColumnType::U32.byte_size(), 4);
        assert_eq!(ColumnType::F32.byte_size(), 4);
        assert_eq!(ColumnType::U64.byte_size(), 8);
        assert_eq!(ColumnType::F64.byte_size(), 8);
    }
}
