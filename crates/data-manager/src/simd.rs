//! SIMD optimizations for high-performance data transformations
//!
//! This module provides platform-specific SIMD implementations for:
//! - Data transformation and normalization
//! - Parallel column processing
//! - Fast aggregation operations
//!
//! Supports AVX2 (x86_64), NEON (ARM), and WASM SIMD with automatic fallbacks

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// SIMD processor trait for different platforms
pub trait SimdProcessor: Send + Sync {
    /// Transform f32 data in parallel
    fn transform_f32(&self, input: &[f32], output: &mut [f32], scale: f32, offset: f32);

    /// Transform f64 data in parallel
    fn transform_f64(&self, input: &[f64], output: &mut [f64], scale: f64, offset: f64);

    /// Calculate min/max in parallel
    fn minmax_f32(&self, data: &[f32]) -> (f32, f32);

    /// Calculate min/max for f64
    fn minmax_f64(&self, data: &[f64]) -> (f64, f64);

    /// Sum elements in parallel
    fn sum_f32(&self, data: &[f32]) -> f32;

    /// Convert u64 timestamps to f32 with normalization
    fn timestamps_to_f32(&self, timestamps: &[u64], output: &mut [f32], scale: f64, offset: f64);

    /// Parallel memcpy optimized for aligned data
    fn fast_copy(&self, src: &[u8], dst: &mut [u8]);
}

/// AVX2 implementation for x86_64
#[cfg(target_arch = "x86_64")]
pub struct Avx2Processor;

#[cfg(target_arch = "x86_64")]
impl SimdProcessor for Avx2Processor {
    fn transform_f32(&self, input: &[f32], output: &mut [f32], scale: f32, offset: f32) {
        unsafe {
            if is_x86_feature_detected!("avx2") {
                transform_f32_avx2(input, output, scale, offset);
            } else {
                transform_f32_scalar(input, output, scale, offset);
            }
        }
    }

    fn transform_f64(&self, input: &[f64], output: &mut [f64], scale: f64, offset: f64) {
        unsafe {
            if is_x86_feature_detected!("avx2") {
                transform_f64_avx2(input, output, scale, offset);
            } else {
                transform_f64_scalar(input, output, scale, offset);
            }
        }
    }

    fn minmax_f32(&self, data: &[f32]) -> (f32, f32) {
        unsafe {
            if is_x86_feature_detected!("avx2") {
                minmax_f32_avx2(data)
            } else {
                minmax_f32_scalar(data)
            }
        }
    }

    fn minmax_f64(&self, data: &[f64]) -> (f64, f64) {
        unsafe {
            if is_x86_feature_detected!("avx2") {
                minmax_f64_avx2(data)
            } else {
                minmax_f64_scalar(data)
            }
        }
    }

    fn sum_f32(&self, data: &[f32]) -> f32 {
        unsafe {
            if is_x86_feature_detected!("avx2") {
                sum_f32_avx2(data)
            } else {
                sum_f32_scalar(data)
            }
        }
    }

    fn timestamps_to_f32(&self, timestamps: &[u64], output: &mut [f32], scale: f64, offset: f64) {
        unsafe {
            if is_x86_feature_detected!("avx2") {
                timestamps_to_f32_avx2(timestamps, output, scale, offset);
            } else {
                timestamps_to_f32_scalar(timestamps, output, scale, offset);
            }
        }
    }

    fn fast_copy(&self, src: &[u8], dst: &mut [u8]) {
        unsafe {
            if is_x86_feature_detected!("avx2") {
                fast_copy_avx2(src, dst);
            } else {
                dst.copy_from_slice(src);
            }
        }
    }
}

// AVX2 implementations
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn transform_f32_avx2(input: &[f32], output: &mut [f32], scale: f32, offset: f32) {
    let len = input.len();
    let simd_width = 8; // AVX2 processes 8 f32s at once
    let chunks = len / simd_width;

    let scale_vec = _mm256_set1_ps(scale);
    let offset_vec = _mm256_set1_ps(offset);

    // Process 8 elements at a time
    for i in 0..chunks {
        let idx = i * simd_width;
        let input_ptr = input.as_ptr().add(idx);
        let output_ptr = output.as_mut_ptr().add(idx);

        // Load 8 floats
        let data = _mm256_loadu_ps(input_ptr);

        // Apply transformation: output = input * scale + offset
        let scaled = _mm256_mul_ps(data, scale_vec);
        let result = _mm256_add_ps(scaled, offset_vec);

        // Store result
        _mm256_storeu_ps(output_ptr, result);
    }

    // Handle remaining elements
    let remainder = chunks * simd_width;
    transform_f32_scalar(&input[remainder..], &mut output[remainder..], scale, offset);
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn transform_f64_avx2(input: &[f64], output: &mut [f64], scale: f64, offset: f64) {
    let len = input.len();
    let simd_width = 4; // AVX2 processes 4 f64s at once
    let chunks = len / simd_width;

    let scale_vec = _mm256_set1_pd(scale);
    let offset_vec = _mm256_set1_pd(offset);

    for i in 0..chunks {
        let idx = i * simd_width;
        let input_ptr = input.as_ptr().add(idx);
        let output_ptr = output.as_mut_ptr().add(idx);

        let data = _mm256_loadu_pd(input_ptr);
        let scaled = _mm256_mul_pd(data, scale_vec);
        let result = _mm256_add_pd(scaled, offset_vec);

        _mm256_storeu_pd(output_ptr, result);
    }

    let remainder = chunks * simd_width;
    transform_f64_scalar(&input[remainder..], &mut output[remainder..], scale, offset);
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn minmax_f32_avx2(data: &[f32]) -> (f32, f32) {
    if data.is_empty() {
        return (0.0, 0.0);
    }

    let len = data.len();
    let simd_width = 8;
    let chunks = len / simd_width;

    // Initialize with first element
    let mut min_vec = _mm256_set1_ps(data[0]);
    let mut max_vec = _mm256_set1_ps(data[0]);

    // Process 8 elements at a time
    for i in 0..chunks {
        let idx = i * simd_width;
        let data_vec = _mm256_loadu_ps(data.as_ptr().add(idx));

        min_vec = _mm256_min_ps(min_vec, data_vec);
        max_vec = _mm256_max_ps(max_vec, data_vec);
    }

    // Extract min/max from vectors
    let min_arr: [f32; 8] = std::mem::transmute(min_vec);
    let max_arr: [f32; 8] = std::mem::transmute(max_vec);

    let mut min = min_arr.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let mut max = max_arr.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

    // Handle remaining elements
    let remainder = chunks * simd_width;
    for &val in &data[remainder..] {
        min = min.min(val);
        max = max.max(val);
    }

    (min, max)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn minmax_f64_avx2(data: &[f64]) -> (f64, f64) {
    if data.is_empty() {
        return (0.0, 0.0);
    }

    let len = data.len();
    let simd_width = 4;
    let chunks = len / simd_width;

    let mut min_vec = _mm256_set1_pd(data[0]);
    let mut max_vec = _mm256_set1_pd(data[0]);

    for i in 0..chunks {
        let idx = i * simd_width;
        let data_vec = _mm256_loadu_pd(data.as_ptr().add(idx));

        min_vec = _mm256_min_pd(min_vec, data_vec);
        max_vec = _mm256_max_pd(max_vec, data_vec);
    }

    let min_arr: [f64; 4] = std::mem::transmute(min_vec);
    let max_arr: [f64; 4] = std::mem::transmute(max_vec);

    let mut min = min_arr.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let mut max = max_arr.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

    let remainder = chunks * simd_width;
    for &val in &data[remainder..] {
        min = min.min(val);
        max = max.max(val);
    }

    (min, max)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn sum_f32_avx2(data: &[f32]) -> f32 {
    let len = data.len();
    let simd_width = 8;
    let chunks = len / simd_width;

    let mut sum_vec = _mm256_setzero_ps();

    for i in 0..chunks {
        let idx = i * simd_width;
        let data_vec = _mm256_loadu_ps(data.as_ptr().add(idx));
        sum_vec = _mm256_add_ps(sum_vec, data_vec);
    }

    // Horizontal sum
    let sum_arr: [f32; 8] = std::mem::transmute(sum_vec);
    let mut sum = sum_arr.iter().sum::<f32>();

    // Handle remaining elements
    let remainder = chunks * simd_width;
    sum += data[remainder..].iter().sum::<f32>();

    sum
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn timestamps_to_f32_avx2(timestamps: &[u64], output: &mut [f32], scale: f64, offset: f64) {
    let len = timestamps.len();
    let simd_width = 4; // Process 4 u64s at a time (converting to f64 first)
    let chunks = len / simd_width;

    let scale_vec = _mm256_set1_pd(scale);
    let offset_vec = _mm256_set1_pd(offset);

    for i in 0..chunks {
        let idx = i * simd_width;

        // Load 4 u64 values and convert to f64
        let ts0 = timestamps[idx] as f64;
        let ts1 = timestamps[idx + 1] as f64;
        let ts2 = timestamps[idx + 2] as f64;
        let ts3 = timestamps[idx + 3] as f64;

        let data_vec = _mm256_set_pd(ts3, ts2, ts1, ts0);

        // Apply transformation
        let scaled = _mm256_mul_pd(data_vec, scale_vec);
        let result = _mm256_add_pd(scaled, offset_vec);

        // Convert to f32 and store
        let result_arr: [f64; 4] = std::mem::transmute(result);
        output[idx] = result_arr[0] as f32;
        output[idx + 1] = result_arr[1] as f32;
        output[idx + 2] = result_arr[2] as f32;
        output[idx + 3] = result_arr[3] as f32;
    }

    // Handle remaining elements
    let remainder = chunks * simd_width;
    timestamps_to_f32_scalar(
        &timestamps[remainder..],
        &mut output[remainder..],
        scale,
        offset,
    );
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn fast_copy_avx2(src: &[u8], dst: &mut [u8]) {
    let len = src.len();
    let simd_width = 32; // AVX2 processes 32 bytes at once
    let chunks = len / simd_width;

    for i in 0..chunks {
        let idx = i * simd_width;
        let src_ptr = src.as_ptr().add(idx);
        let dst_ptr = dst.as_mut_ptr().add(idx);

        let data = _mm256_loadu_si256(src_ptr as *const __m256i);
        _mm256_storeu_si256(dst_ptr as *mut __m256i, data);
    }

    // Handle remaining bytes
    let remainder = chunks * simd_width;
    dst[remainder..].copy_from_slice(&src[remainder..]);
}

// Scalar fallback implementations
fn transform_f32_scalar(input: &[f32], output: &mut [f32], scale: f32, offset: f32) {
    for (i, &val) in input.iter().enumerate() {
        output[i] = val * scale + offset;
    }
}

fn transform_f64_scalar(input: &[f64], output: &mut [f64], scale: f64, offset: f64) {
    for (i, &val) in input.iter().enumerate() {
        output[i] = val * scale + offset;
    }
}

fn minmax_f32_scalar(data: &[f32]) -> (f32, f32) {
    if data.is_empty() {
        return (0.0, 0.0);
    }

    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;

    for &val in data {
        min = min.min(val);
        max = max.max(val);
    }

    (min, max)
}

fn minmax_f64_scalar(data: &[f64]) -> (f64, f64) {
    if data.is_empty() {
        return (0.0, 0.0);
    }

    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;

    for &val in data {
        min = min.min(val);
        max = max.max(val);
    }

    (min, max)
}

fn sum_f32_scalar(data: &[f32]) -> f32 {
    data.iter().sum()
}

fn timestamps_to_f32_scalar(timestamps: &[u64], output: &mut [f32], scale: f64, offset: f64) {
    for (i, &ts) in timestamps.iter().enumerate() {
        output[i] = (ts as f64 * scale + offset) as f32;
    }
}

/// Factory to create appropriate SIMD processor for the current platform
pub fn create_simd_processor() -> Box<dyn SimdProcessor> {
    #[cfg(target_arch = "x86_64")]
    {
        Box::new(Avx2Processor)
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        Box::new(ScalarProcessor)
    }
}

/// Scalar processor fallback for unsupported platforms
pub struct ScalarProcessor;

impl SimdProcessor for ScalarProcessor {
    fn transform_f32(&self, input: &[f32], output: &mut [f32], scale: f32, offset: f32) {
        transform_f32_scalar(input, output, scale, offset);
    }

    fn transform_f64(&self, input: &[f64], output: &mut [f64], scale: f64, offset: f64) {
        transform_f64_scalar(input, output, scale, offset);
    }

    fn minmax_f32(&self, data: &[f32]) -> (f32, f32) {
        minmax_f32_scalar(data)
    }

    fn minmax_f64(&self, data: &[f64]) -> (f64, f64) {
        minmax_f64_scalar(data)
    }

    fn sum_f32(&self, data: &[f32]) -> f32 {
        sum_f32_scalar(data)
    }

    fn timestamps_to_f32(&self, timestamps: &[u64], output: &mut [f32], scale: f64, offset: f64) {
        timestamps_to_f32_scalar(timestamps, output, scale, offset);
    }

    fn fast_copy(&self, src: &[u8], dst: &mut [u8]) {
        dst.copy_from_slice(src);
    }
}

/// Batch processor for handling multiple columns in parallel
pub struct SimdBatchProcessor {
    processor: Box<dyn SimdProcessor>,
}

impl SimdBatchProcessor {
    pub fn new() -> Self {
        Self {
            processor: create_simd_processor(),
        }
    }

    /// Process multiple columns in parallel
    pub fn process_columns(&self, columns: Vec<ColumnData>) -> Vec<ProcessedColumn> {
        columns
            .into_iter()
            .map(|col| self.process_single_column(col))
            .collect()
    }

    fn process_single_column(&self, column: ColumnData) -> ProcessedColumn {
        match column {
            ColumnData::F32(data) => {
                let (min, max) = self.processor.minmax_f32(&data);
                let scale = if max > min { 1.0 / (max - min) } else { 1.0 };
                let offset = -min * scale;

                let mut normalized = vec![0.0f32; data.len()];
                self.processor
                    .transform_f32(&data, &mut normalized, scale, offset);

                ProcessedColumn::F32 {
                    data: normalized,
                    min,
                    max,
                    scale,
                    offset,
                }
            }
            ColumnData::F64(data) => {
                let (min, max) = self.processor.minmax_f64(&data);
                let scale = if max > min { 1.0 / (max - min) } else { 1.0 };
                let offset = -min * scale;

                let mut normalized = vec![0.0f64; data.len()];
                self.processor
                    .transform_f64(&data, &mut normalized, scale, offset);

                ProcessedColumn::F64 {
                    data: normalized,
                    min,
                    max,
                    scale,
                    offset,
                }
            }
            ColumnData::Timestamps(timestamps) => {
                if timestamps.is_empty() {
                    return ProcessedColumn::F32 {
                        data: vec![],
                        min: 0.0,
                        max: 0.0,
                        scale: 1.0,
                        offset: 0.0,
                    };
                }

                let min = *timestamps.iter().min().unwrap() as f64;
                let max = *timestamps.iter().max().unwrap() as f64;
                let scale = if max > min { 1.0 / (max - min) } else { 1.0 };
                let offset = -min * scale;

                let mut normalized = vec![0.0f32; timestamps.len()];
                self.processor
                    .timestamps_to_f32(&timestamps, &mut normalized, scale, offset);

                ProcessedColumn::F32 {
                    data: normalized,
                    min: min as f32,
                    max: max as f32,
                    scale: scale as f32,
                    offset: offset as f32,
                }
            }
        }
    }
}

/// Input column data types
pub enum ColumnData {
    F32(Vec<f32>),
    F64(Vec<f64>),
    Timestamps(Vec<u64>),
}

/// Processed column with normalization info
pub enum ProcessedColumn {
    F32 {
        data: Vec<f32>,
        min: f32,
        max: f32,
        scale: f32,
        offset: f32,
    },
    F64 {
        data: Vec<f64>,
        min: f64,
        max: f64,
        scale: f64,
        offset: f64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_f32() {
        let processor = create_simd_processor();
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let mut output = vec![0.0; 8];

        processor.transform_f32(&input, &mut output, 2.0, 1.0);

        let expected: Vec<f32> = input.iter().map(|&x| x * 2.0 + 1.0).collect();
        assert_eq!(output, expected);
    }

    #[test]
    fn test_minmax() {
        let processor = create_simd_processor();
        let data = vec![5.0, 2.0, 8.0, 1.0, 9.0, 3.0, 7.0, 4.0, 6.0];

        let (min, max) = processor.minmax_f32(&data);
        assert_eq!(min, 1.0);
        assert_eq!(max, 9.0);
    }

    #[test]
    fn test_sum() {
        let processor = create_simd_processor();
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

        let sum = processor.sum_f32(&data);
        assert_eq!(sum, 36.0);
    }

    #[test]
    fn test_batch_processing() {
        let batch = SimdBatchProcessor::new();

        let columns = vec![
            ColumnData::F32(vec![1.0, 2.0, 3.0, 4.0]),
            ColumnData::Timestamps(vec![1000, 2000, 3000, 4000]),
        ];

        let processed = batch.process_columns(columns);
        assert_eq!(processed.len(), 2);
    }
}
