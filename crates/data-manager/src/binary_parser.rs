//! Binary data parsing with SIMD optimizations

use shared_types::{ParsedData, DataMetadata};
use std::collections::HashMap;

/// Parse binary data from server into structured format
pub fn parse_binary_data(
    header: DataResponseHeader,
    binary_data: &[u8],
) -> Result<ParsedData, ParserError> {
    let mut time_data = Vec::new();
    let mut value_data: HashMap<String, Vec<f32>> = HashMap::new();
    
    // Initialize value vectors
    for column in &header.columns {
        if column != "time" {
            value_data.insert(column.clone(), Vec::new());
        }
    }

    let bytes_per_row = header.columns.len() * 4; // 4 bytes per value
    let expected_bytes = header.row_count * bytes_per_row;
    
    if binary_data.len() != expected_bytes {
        return Err(ParserError::InvalidDataSize {
            expected: expected_bytes,
            actual: binary_data.len(),
        });
    }

    // Parse row by row
    for row_idx in 0..header.row_count {
        let row_start = row_idx * bytes_per_row;
        
        for (col_idx, column) in header.columns.iter().enumerate() {
            let offset = row_start + col_idx * 4;
            let bytes = &binary_data[offset..offset + 4];
            
            if column == "time" {
                let timestamp = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                time_data.push(timestamp);
            } else {
                let value = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                value_data.get_mut(column).unwrap().push(value);
            }
        }
    }

    Ok(ParsedData {
        time_data,
        value_data,
        metadata: DataMetadata {
            symbol: header.symbol,
            start_time: header.start_time,
            end_time: header.end_time,
            columns: header.columns,
            row_count: header.row_count,
        },
    })
}

/// SIMD-optimized binary search for sorted timestamp arrays
#[cfg(target_arch = "wasm32")]
pub fn binary_search_timestamp(data: &[u8], target: u32) -> Option<usize> {
    if data.len() < 4 {
        return None;
    }

    let mut left = 0;
    let mut right = data.len() / 4 - 1;

    while left <= right {
        let mid = left + (right - left) / 2;
        let offset = mid * 4;
        
        let timestamp = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);

        match timestamp.cmp(&target) {
            std::cmp::Ordering::Equal => return Some(mid),
            std::cmp::Ordering::Less => left = mid + 1,
            std::cmp::Ordering::Greater => {
                if mid == 0 {
                    break;
                }
                right = mid - 1;
            }
        }
    }

    // Return the insertion point
    Some(left)
}

/// Data response header from server
#[derive(Debug, Clone)]
pub struct DataResponseHeader {
    pub symbol: String,
    pub columns: Vec<String>,
    pub start_time: u64,
    pub end_time: u64,
    pub row_count: usize,
}

/// Parser errors
#[derive(Debug, thiserror::Error)]
pub enum ParserError {
    #[error("Invalid data size: expected {expected} bytes, got {actual}")]
    InvalidDataSize { expected: usize, actual: usize },
    
    #[error("Invalid column configuration")]
    InvalidColumns,
    
    #[error("Parsing error: {0}")]
    ParseError(String),
}

/// Optimized batch parser for large datasets
pub struct BatchParser {
    chunk_size: usize,
}

impl BatchParser {
    pub fn new(chunk_size: usize) -> Self {
        Self { chunk_size }
    }

    /// Parse data in chunks for better memory efficiency
    pub fn parse_chunked(
        &self,
        header: DataResponseHeader,
        binary_data: &[u8],
        mut callback: impl FnMut(ParsedData) -> Result<(), ParserError>,
    ) -> Result<(), ParserError> {
        let bytes_per_row = header.columns.len() * 4;
        let total_rows = header.row_count;
        let mut processed_rows = 0;

        while processed_rows < total_rows {
            let chunk_rows = (total_rows - processed_rows).min(self.chunk_size);
            let chunk_start = processed_rows * bytes_per_row;
            let chunk_end = chunk_start + chunk_rows * bytes_per_row;
            
            let chunk_data = &binary_data[chunk_start..chunk_end];
            
            // Parse chunk
            let chunk_header = DataResponseHeader {
                symbol: header.symbol.clone(),
                columns: header.columns.clone(),
                start_time: header.start_time,
                end_time: header.end_time,
                row_count: chunk_rows,
            };
            
            let parsed_chunk = parse_binary_data(chunk_header, chunk_data)?;
            callback(parsed_chunk)?;
            
            processed_rows += chunk_rows;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_search() {
        let timestamps = vec![100u32, 200, 300, 400, 500];
        let data: Vec<u8> = timestamps
            .iter()
            .flat_map(|t| t.to_le_bytes())
            .collect();

        assert_eq!(binary_search_timestamp(&data, 300), Some(2));
        assert_eq!(binary_search_timestamp(&data, 250), Some(2)); // Insertion point
        assert_eq!(binary_search_timestamp(&data, 50), Some(0));
        assert_eq!(binary_search_timestamp(&data, 600), Some(5));
    }

    #[test]
    fn test_parse_binary_data() {
        let header = DataResponseHeader {
            symbol: "BTC-USD".to_string(),
            columns: vec!["time".to_string(), "price".to_string()],
            start_time: 1000,
            end_time: 2000,
            row_count: 2,
        };

        let mut data = Vec::new();
        // Row 1: time=1000, price=50000.0
        data.extend_from_slice(&1000u32.to_le_bytes());
        data.extend_from_slice(&50000.0f32.to_le_bytes());
        // Row 2: time=2000, price=51000.0
        data.extend_from_slice(&2000u32.to_le_bytes());
        data.extend_from_slice(&51000.0f32.to_le_bytes());

        let parsed = parse_binary_data(header, &data).unwrap();
        
        assert_eq!(parsed.time_data.len(), 2);
        assert_eq!(parsed.time_data[0], 1000);
        assert_eq!(parsed.time_data[1], 2000);
        
        assert_eq!(parsed.value_data["price"].len(), 2);
        assert_eq!(parsed.value_data["price"][0], 50000.0);
        assert_eq!(parsed.value_data["price"][1], 51000.0);
    }
}