//! Test data generation for benchmarks

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::f32::consts::PI;

/// Generate time-series data with various patterns
pub struct DataGenerator {
    rng: StdRng,
}

impl DataGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Generate simple line chart data
    pub fn generate_line_data(&mut self, points: usize) -> Vec<[f32; 2]> {
        let mut data = Vec::with_capacity(points);
        let time_step = 1.0 / points as f32;
        let mut value = 100.0;

        for i in 0..points {
            let time = i as f32 * time_step;

            // Random walk with trend
            value += self.rng.gen_range(-1.0..1.0);
            value += (time * 0.1).sin() * 5.0; // Add some pattern

            data.push([time, value]);
        }

        data
    }

    /// Generate OHLC data for candlestick charts
    pub fn generate_ohlc_data(&mut self, candles: usize) -> Vec<[f32; 5]> {
        let mut data = Vec::with_capacity(candles);
        let mut current_price = 100.0;

        for i in 0..candles {
            let time = i as f32;

            // Generate OHLC values
            let open = current_price;
            let volatility = self.rng.gen_range(0.5..2.0);
            let high = open + self.rng.gen_range(0.0..volatility);
            let low = open - self.rng.gen_range(0.0..volatility);
            let close = self.rng.gen_range(low..high);

            current_price = close;

            data.push([time, open, high, low, close]);
        }

        data
    }

    /// Generate volume data
    pub fn generate_volume_data(&mut self, points: usize) -> Vec<f32> {
        let mut data = Vec::with_capacity(points);
        let base_volume = 1_000_000.0;

        for i in 0..points {
            let time = i as f32 / points as f32;

            // Volume with daily pattern
            let daily_pattern = (time * 2.0 * PI).sin().abs();
            let random_factor = self.rng.gen_range(0.5..1.5);
            let volume = base_volume * daily_pattern * random_factor;

            data.push(volume);
        }

        data
    }

    /// Generate multi-dimensional data
    pub fn generate_multi_column_data(&mut self, rows: usize, columns: usize) -> Vec<Vec<f32>> {
        let mut data = vec![vec![0.0; rows]; columns];

        // Time column
        for i in 0..rows {
            data[0][i] = i as f32;
        }

        // Value columns with different patterns
        for col in 1..columns {
            let frequency = col as f32 * 0.1;
            let amplitude = 10.0 * (col as f32).sqrt();
            let offset = col as f32 * 50.0;

            for row in 0..rows {
                let time = row as f32 / rows as f32;
                let wave = (time * frequency * 2.0 * PI).sin() * amplitude;
                let noise = self.rng.gen_range(-1.0..1.0);
                data[col][row] = offset + wave + noise;
            }
        }

        data
    }

    /// Generate data that tests edge cases
    pub fn generate_edge_case_data(&mut self, points: usize) -> Vec<[f32; 2]> {
        let mut data = Vec::with_capacity(points);

        for i in 0..points {
            let time = i as f32;
            let value = match i % 10 {
                0 => f32::NAN,                            // NaN values
                1 => f32::INFINITY,                       // Infinity
                2 => f32::NEG_INFINITY,                   // Negative infinity
                3 => 0.0,                                 // Zero
                4 => f32::MIN,                            // Min value
                5 => f32::MAX,                            // Max value
                _ => self.rng.gen_range(-1000.0..1000.0), // Normal values
            };

            data.push([time, value]);
        }

        data
    }

    /// Generate binary data similar to server format
    pub fn generate_binary_data(&mut self, points: usize) -> Vec<u8> {
        let float_data = self.generate_line_data(points);
        let mut binary = Vec::with_capacity(points * 8);

        for [time, value] in float_data {
            binary.extend_from_slice(&time.to_le_bytes());
            binary.extend_from_slice(&value.to_le_bytes());
        }

        binary
    }

    /// Generate GPU buffer data
    pub fn generate_gpu_buffer_data(&mut self, points: usize) -> Vec<f32> {
        let mut data = Vec::with_capacity(points * 2);

        for point in self.generate_line_data(points) {
            data.push(point[0]);
            data.push(point[1]);
        }

        data
    }
}

/// Predefined data scenarios
pub struct DataScenarios;

impl DataScenarios {
    /// High-frequency trading data (millions of ticks)
    pub fn hft_scenario(points: usize) -> Vec<[f32; 2]> {
        let mut gen = DataGenerator::new(42);
        gen.generate_line_data(points)
    }

    /// Daily OHLC data (years of history)
    pub fn daily_ohlc_scenario(days: usize) -> Vec<[f32; 5]> {
        let mut gen = DataGenerator::new(42);
        gen.generate_ohlc_data(days)
    }

    /// Real-time streaming scenario
    pub fn streaming_scenario(initial_points: usize) -> (Vec<[f32; 2]>, DataGenerator) {
        let mut gen = DataGenerator::new(42);
        let initial = gen.generate_line_data(initial_points);
        (initial, gen)
    }

    /// Multi-asset correlation data
    pub fn multi_asset_scenario(assets: usize, points: usize) -> Vec<Vec<f32>> {
        let mut gen = DataGenerator::new(42);
        gen.generate_multi_column_data(points, assets + 1) // +1 for time
    }
}
