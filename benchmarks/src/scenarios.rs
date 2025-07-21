//! Benchmark scenarios based on real-world usage

use crate::data_generator::DataGenerator;
use crate::metrics::PerformanceMetrics;
use std::time::{Duration, Instant};

/// Common benchmark scenarios
pub enum BenchmarkScenario {
    /// Basic line chart with varying point counts
    BasicLineChart { points: usize },

    /// Candlestick chart with OHLC data
    CandlestickChart { candles: usize },

    /// Multiple overlays on same chart
    MultipleOverlays { points: usize, overlays: usize },

    /// Rapid zoom in/out operations
    RapidZoom {
        points: usize,
        zoom_iterations: usize,
    },

    /// Continuous panning
    ContinuousPan { points: usize, pan_distance: f32 },

    /// Real-time data streaming
    RealTimeStreaming {
        initial_points: usize,
        updates_per_second: usize,
    },

    /// Memory pressure test
    MemoryPressure {
        charts: usize,
        points_per_chart: usize,
    },

    /// Multi-timeframe analysis
    MultiTimeframe { timeframes: Vec<usize> },

    /// Edge case handling
    EdgeCases { points: usize },
}

impl BenchmarkScenario {
    /// Get a human-readable name for the scenario
    pub fn name(&self) -> String {
        match self {
            Self::BasicLineChart { points } => format!("Basic Line Chart ({} points)", points),
            Self::CandlestickChart { candles } => {
                format!("Candlestick Chart ({} candles)", candles)
            }
            Self::MultipleOverlays { points, overlays } => {
                format!(
                    "Multiple Overlays ({} points, {} overlays)",
                    points, overlays
                )
            }
            Self::RapidZoom {
                points,
                zoom_iterations,
            } => {
                format!(
                    "Rapid Zoom ({} points, {} iterations)",
                    points, zoom_iterations
                )
            }
            Self::ContinuousPan {
                points,
                pan_distance,
            } => {
                format!(
                    "Continuous Pan ({} points, {} distance)",
                    points, pan_distance
                )
            }
            Self::RealTimeStreaming {
                initial_points,
                updates_per_second,
            } => {
                format!(
                    "Real-time Streaming ({} initial, {} updates/sec)",
                    initial_points, updates_per_second
                )
            }
            Self::MemoryPressure {
                charts,
                points_per_chart,
            } => {
                format!(
                    "Memory Pressure ({} charts, {} points each)",
                    charts, points_per_chart
                )
            }
            Self::MultiTimeframe { timeframes } => {
                format!("Multi-timeframe ({} timeframes)", timeframes.len())
            }
            Self::EdgeCases { points } => format!("Edge Cases ({} points)", points),
        }
    }

    /// Generate test data for the scenario
    pub fn generate_data(&self) -> ScenarioData {
        let mut gen = DataGenerator::new(42);

        match self {
            Self::BasicLineChart { points } => {
                ScenarioData::LineData(gen.generate_line_data(*points))
            }
            Self::CandlestickChart { candles } => {
                ScenarioData::OhlcData(gen.generate_ohlc_data(*candles))
            }
            Self::MultipleOverlays { points, overlays } => {
                let mut data = vec![];
                for _ in 0..*overlays {
                    data.push(gen.generate_line_data(*points));
                }
                ScenarioData::MultiLineData(data)
            }
            Self::RapidZoom { points, .. } => {
                ScenarioData::LineData(gen.generate_line_data(*points))
            }
            Self::ContinuousPan { points, .. } => {
                ScenarioData::LineData(gen.generate_line_data(*points))
            }
            Self::RealTimeStreaming { initial_points, .. } => {
                ScenarioData::LineData(gen.generate_line_data(*initial_points))
            }
            Self::MemoryPressure {
                charts,
                points_per_chart,
            } => {
                let mut data = vec![];
                for _ in 0..*charts {
                    data.push(gen.generate_line_data(*points_per_chart));
                }
                ScenarioData::MultiLineData(data)
            }
            Self::MultiTimeframe { timeframes } => {
                let mut data = vec![];
                for &tf in timeframes {
                    data.push(gen.generate_line_data(tf));
                }
                ScenarioData::MultiLineData(data)
            }
            Self::EdgeCases { points } => {
                ScenarioData::LineData(gen.generate_edge_case_data(*points))
            }
        }
    }
}

/// Data types for different scenarios
pub enum ScenarioData {
    LineData(Vec<[f32; 2]>),
    OhlcData(Vec<[f32; 5]>),
    MultiLineData(Vec<Vec<[f32; 2]>>),
    VolumeData(Vec<f32>),
}

/// Standard benchmark scenarios
pub struct StandardScenarios;

impl StandardScenarios {
    /// Small dataset (suitable for mobile)
    pub fn small() -> Vec<BenchmarkScenario> {
        vec![
            BenchmarkScenario::BasicLineChart { points: 1_000 },
            BenchmarkScenario::BasicLineChart { points: 10_000 },
            BenchmarkScenario::CandlestickChart { candles: 500 },
            BenchmarkScenario::MultipleOverlays {
                points: 1_000,
                overlays: 3,
            },
        ]
    }

    /// Medium dataset (typical desktop usage)
    pub fn medium() -> Vec<BenchmarkScenario> {
        vec![
            BenchmarkScenario::BasicLineChart { points: 100_000 },
            BenchmarkScenario::BasicLineChart { points: 1_000_000 },
            BenchmarkScenario::CandlestickChart { candles: 10_000 },
            BenchmarkScenario::MultipleOverlays {
                points: 100_000,
                overlays: 5,
            },
            BenchmarkScenario::RapidZoom {
                points: 100_000,
                zoom_iterations: 10,
            },
            BenchmarkScenario::ContinuousPan {
                points: 100_000,
                pan_distance: 0.5,
            },
        ]
    }

    /// Large dataset (high-end workstation)
    pub fn large() -> Vec<BenchmarkScenario> {
        vec![
            BenchmarkScenario::BasicLineChart { points: 10_000_000 },
            BenchmarkScenario::BasicLineChart {
                points: 100_000_000,
            },
            BenchmarkScenario::CandlestickChart { candles: 100_000 },
            BenchmarkScenario::MultipleOverlays {
                points: 10_000_000,
                overlays: 10,
            },
            BenchmarkScenario::MemoryPressure {
                charts: 10,
                points_per_chart: 1_000_000,
            },
        ]
    }

    /// Extreme dataset (stress testing)
    pub fn extreme() -> Vec<BenchmarkScenario> {
        vec![
            BenchmarkScenario::BasicLineChart {
                points: 1_000_000_000,
            },
            BenchmarkScenario::MemoryPressure {
                charts: 50,
                points_per_chart: 10_000_000,
            },
            BenchmarkScenario::MultiTimeframe {
                timeframes: vec![60, 300, 900, 3600, 86400], // 1min, 5min, 15min, 1h, 1d
            },
        ]
    }

    /// Interactive usage patterns
    pub fn interactive() -> Vec<BenchmarkScenario> {
        vec![
            BenchmarkScenario::RapidZoom {
                points: 1_000_000,
                zoom_iterations: 50,
            },
            BenchmarkScenario::ContinuousPan {
                points: 1_000_000,
                pan_distance: 2.0,
            },
            BenchmarkScenario::RealTimeStreaming {
                initial_points: 100_000,
                updates_per_second: 60,
            },
        ]
    }

    /// Edge cases and error conditions
    pub fn edge_cases() -> Vec<BenchmarkScenario> {
        vec![
            BenchmarkScenario::EdgeCases { points: 10_000 },
            BenchmarkScenario::BasicLineChart { points: 0 }, // Empty chart
            BenchmarkScenario::BasicLineChart { points: 1 }, // Single point
            BenchmarkScenario::MultipleOverlays {
                points: 100,
                overlays: 100,
            }, // Many overlays
        ]
    }
}

/// Scenario runner that simulates user interactions
pub struct ScenarioRunner;

impl ScenarioRunner {
    /// Simulate zoom interaction
    pub fn simulate_zoom(zoom_level: f32, iterations: usize) -> Vec<f32> {
        let mut levels = vec![];
        let mut current = 1.0;

        for i in 0..iterations {
            // Zoom in and out pattern
            if i % 2 == 0 {
                current *= zoom_level;
            } else {
                current /= zoom_level;
            }
            levels.push(current);
        }

        levels
    }

    /// Simulate pan interaction
    pub fn simulate_pan(pan_distance: f32, duration: Duration) -> Vec<f32> {
        let steps = (duration.as_secs_f32() * 60.0) as usize; // 60 FPS
        let mut positions = vec![];
        let step_size = pan_distance / steps as f32;

        for i in 0..steps {
            positions.push(i as f32 * step_size);
        }

        positions
    }

    /// Simulate real-time data updates
    pub fn simulate_streaming(updates_per_second: usize, duration: Duration) -> Vec<Instant> {
        let total_updates = (duration.as_secs_f32() * updates_per_second as f32) as usize;
        let mut update_times = vec![];
        let update_interval = Duration::from_secs_f32(1.0 / updates_per_second as f32);

        let mut current_time = Instant::now();
        for _ in 0..total_updates {
            update_times.push(current_time);
            current_time += update_interval;
        }

        update_times
    }
}
