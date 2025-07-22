//! Phase 3 Advanced Overlays and Technical Indicators benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::VecDeque;

/// Mock price data
#[derive(Clone, Copy)]
struct PriceData {
    timestamp: u64,
    open: f32,
    high: f32,
    low: f32,
    close: f32,
    volume: f32,
}

/// Technical indicator calculations
struct TechnicalIndicators;

impl TechnicalIndicators {
    /// Calculate Simple Moving Average
    fn sma(prices: &[f32], period: usize) -> Vec<f32> {
        let mut sma_values = Vec::with_capacity(prices.len());
        let mut sum = 0.0;
        let mut window = VecDeque::with_capacity(period);

        for &price in prices {
            window.push_back(price);
            sum += price;

            if window.len() > period {
                sum -= window.pop_front().unwrap();
            }

            if window.len() == period {
                sma_values.push(sum / period as f32);
            } else {
                sma_values.push(f32::NAN);
            }
        }

        sma_values
    }

    /// Calculate Exponential Moving Average
    fn ema(prices: &[f32], period: usize) -> Vec<f32> {
        let mut ema_values = Vec::with_capacity(prices.len());
        let multiplier = 2.0 / (period as f32 + 1.0);

        if prices.is_empty() {
            return ema_values;
        }

        // Start with SMA for the first value
        let mut ema = prices.iter().take(period).sum::<f32>() / period as f32;

        for (i, &price) in prices.iter().enumerate() {
            if i < period - 1 {
                ema_values.push(f32::NAN);
            } else if i == period - 1 {
                ema_values.push(ema);
            } else {
                ema = (price - ema) * multiplier + ema;
                ema_values.push(ema);
            }
        }

        ema_values
    }

    /// Calculate Bollinger Bands
    fn bollinger_bands(
        prices: &[f32],
        period: usize,
        std_dev: f32,
    ) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        let sma = Self::sma(prices, period);
        let mut upper = Vec::with_capacity(prices.len());
        let mut lower = Vec::with_capacity(prices.len());

        for i in 0..prices.len() {
            if i < period - 1 {
                upper.push(f32::NAN);
                lower.push(f32::NAN);
            } else {
                // Calculate standard deviation
                let mean = sma[i];
                let variance: f32 = prices[i - period + 1..=i]
                    .iter()
                    .map(|&x| (x - mean).powi(2))
                    .sum::<f32>()
                    / period as f32;
                let std = variance.sqrt();

                upper.push(mean + std_dev * std);
                lower.push(mean - std_dev * std);
            }
        }

        (upper, sma, lower)
    }

    /// Calculate RSI (Relative Strength Index)
    fn rsi(prices: &[f32], period: usize) -> Vec<f32> {
        let mut rsi_values = Vec::with_capacity(prices.len());

        if prices.len() < period + 1 {
            return vec![f32::NAN; prices.len()];
        }

        let mut gains = Vec::new();
        let mut losses = Vec::new();

        for i in 1..prices.len() {
            let change = prices[i] - prices[i - 1];
            if change > 0.0 {
                gains.push(change);
                losses.push(0.0);
            } else {
                gains.push(0.0);
                losses.push(-change);
            }
        }

        let mut avg_gain = gains.iter().take(period).sum::<f32>() / period as f32;
        let mut avg_loss = losses.iter().take(period).sum::<f32>() / period as f32;

        // Fill initial values with NaN
        for _ in 0..period {
            rsi_values.push(f32::NAN);
        }

        // Calculate RSI values
        for i in period..gains.len() {
            avg_gain = (avg_gain * (period - 1) as f32 + gains[i]) / period as f32;
            avg_loss = (avg_loss * (period - 1) as f32 + losses[i]) / period as f32;

            let rs = if avg_loss > 0.0 {
                avg_gain / avg_loss
            } else {
                100.0
            };
            let rsi = 100.0 - (100.0 / (1.0 + rs));
            rsi_values.push(rsi);
        }

        rsi_values
    }

    /// Calculate MACD (Moving Average Convergence Divergence)
    fn macd(
        prices: &[f32],
        fast: usize,
        slow: usize,
        signal: usize,
    ) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        let ema_fast = Self::ema(prices, fast);
        let ema_slow = Self::ema(prices, slow);

        let mut macd_line = Vec::with_capacity(prices.len());
        for i in 0..prices.len() {
            if i < slow - 1 {
                macd_line.push(f32::NAN);
            } else {
                macd_line.push(ema_fast[i] - ema_slow[i]);
            }
        }

        let signal_line = Self::ema(&macd_line, signal);

        let mut histogram = Vec::with_capacity(prices.len());
        for i in 0..prices.len() {
            if macd_line[i].is_nan() || signal_line[i].is_nan() {
                histogram.push(f32::NAN);
            } else {
                histogram.push(macd_line[i] - signal_line[i]);
            }
        }

        (macd_line, signal_line, histogram)
    }
}

/// Benchmark technical indicators
fn benchmark_technical_indicators(c: &mut Criterion) {
    let mut group = c.benchmark_group("technical_indicators");

    for &data_points in &[1_000, 10_000, 100_000, 1_000_000] {
        group.throughput(Throughput::Elements(data_points as u64));

        let prices: Vec<f32> = (0..data_points)
            .map(|i| 100.0 + (i as f32 * 0.01).sin() * 10.0)
            .collect();

        // Benchmark Simple Moving Average
        group.bench_with_input(
            BenchmarkId::new("sma_20", data_points),
            &prices,
            |b, prices| {
                b.iter(|| black_box(TechnicalIndicators::sma(prices, 20)));
            },
        );

        // Benchmark Exponential Moving Average
        group.bench_with_input(
            BenchmarkId::new("ema_20", data_points),
            &prices,
            |b, prices| {
                b.iter(|| black_box(TechnicalIndicators::ema(prices, 20)));
            },
        );

        // Benchmark Bollinger Bands
        group.bench_with_input(
            BenchmarkId::new("bollinger_bands", data_points),
            &prices,
            |b, prices| {
                b.iter(|| black_box(TechnicalIndicators::bollinger_bands(prices, 20, 2.0)));
            },
        );

        // Benchmark RSI
        group.bench_with_input(
            BenchmarkId::new("rsi_14", data_points),
            &prices,
            |b, prices| {
                b.iter(|| black_box(TechnicalIndicators::rsi(prices, 14)));
            },
        );

        // Benchmark MACD
        group.bench_with_input(
            BenchmarkId::new("macd", data_points),
            &prices,
            |b, prices| {
                b.iter(|| black_box(TechnicalIndicators::macd(prices, 12, 26, 9)));
            },
        );
    }

    group.finish();
}

/// Annotation system structures
#[derive(Clone)]
struct Annotation {
    x: f32,
    y: f32,
    text: String,
    style: AnnotationStyle,
}

#[derive(Clone)]
struct AnnotationStyle {
    font_size: f32,
    color: [f32; 4],
    background: Option<[f32; 4]>,
    border: Option<(f32, [f32; 4])>,
}

/// Benchmark annotation rendering
fn benchmark_annotations(c: &mut Criterion) {
    let mut group = c.benchmark_group("annotations");

    for &annotation_count in &[10, 50, 100, 500] {
        let annotations = generate_annotations(annotation_count);

        // Benchmark text layout calculation
        group.bench_with_input(
            BenchmarkId::new("text_layout", annotation_count),
            &annotations,
            |b, annotations| {
                b.iter(|| {
                    let mut total_width = 0.0;
                    for annotation in annotations {
                        // Simulate text measurement
                        let width = annotation.text.len() as f32 * annotation.style.font_size * 0.6;
                        let height = annotation.style.font_size * 1.2;
                        total_width += width + height;
                    }
                    black_box(total_width)
                });
            },
        );

        // Benchmark annotation collision detection
        group.bench_with_input(
            BenchmarkId::new("collision_detection", annotation_count),
            &annotations,
            |b, annotations| {
                b.iter(|| {
                    let mut collisions = 0;
                    for i in 0..annotations.len() {
                        for j in i + 1..annotations.len() {
                            let dx = annotations[i].x - annotations[j].x;
                            let dy = annotations[i].y - annotations[j].y;
                            if dx.abs() < 50.0 && dy.abs() < 20.0 {
                                collisions += 1;
                            }
                        }
                    }
                    black_box(collisions)
                });
            },
        );

        // Benchmark annotation rendering preparation
        group.bench_with_input(
            BenchmarkId::new("render_preparation", annotation_count),
            &annotations,
            |b, annotations| {
                b.iter(|| {
                    let mut vertices = Vec::with_capacity(annotations.len() * 24);
                    for annotation in annotations {
                        // Generate quad for background
                        if annotation.style.background.is_some() {
                            let width =
                                annotation.text.len() as f32 * annotation.style.font_size * 0.6;
                            let height = annotation.style.font_size * 1.2;

                            vertices.extend_from_slice(&[
                                annotation.x,
                                annotation.y,
                                annotation.x + width,
                                annotation.y,
                                annotation.x + width,
                                annotation.y + height,
                                annotation.x,
                                annotation.y + height,
                            ]);
                        }
                    }
                    black_box(vertices)
                });
            },
        );
    }

    group.finish();
}

/// Custom shader compilation simulation
fn benchmark_custom_shaders(c: &mut Criterion) {
    let mut group = c.benchmark_group("custom_shaders");

    let shader_sizes = vec![
        ("small", 100),  // 100 lines
        ("medium", 500), // 500 lines
        ("large", 1000), // 1000 lines
    ];

    for (name, lines) in shader_sizes {
        let shader_code = generate_shader_code(lines);

        // Benchmark shader parsing
        group.bench_with_input(
            BenchmarkId::new("shader_parsing", name),
            &shader_code,
            |b, code| {
                b.iter(|| parse_shader_code(code));
            },
        );

        // Benchmark shader validation
        group.bench_with_input(
            BenchmarkId::new("shader_validation", name),
            &shader_code,
            |b, code| {
                b.iter(|| validate_shader_safety(code));
            },
        );
    }

    group.finish();
}

// Helper functions
fn generate_annotations(count: usize) -> Vec<Annotation> {
    (0..count)
        .map(|i| Annotation {
            x: (i as f32 * 100.0) % 1000.0,
            y: (i as f32 * 50.0) % 500.0,
            text: format!("Annotation {}", i),
            style: AnnotationStyle {
                font_size: 12.0,
                color: [1.0, 1.0, 1.0, 1.0],
                background: Some([0.0, 0.0, 0.0, 0.8]),
                border: Some((1.0, [1.0, 1.0, 1.0, 1.0])),
            },
        })
        .collect()
}

fn generate_shader_code(lines: usize) -> String {
    let mut code = String::new();
    code.push_str("struct VertexOutput {\n");
    code.push_str("    @builtin(position) position: vec4<f32>,\n");
    code.push_str("    @location(0) color: vec4<f32>,\n");
    code.push_str("};\n\n");

    for i in 0..lines {
        code.push_str(&format!("// Line {} of shader code\n", i));
        if i % 10 == 0 {
            code.push_str("fn helper_function_");
            code.push_str(&i.to_string());
            code.push_str("(x: f32) -> f32 {\n");
            code.push_str("    return x * 2.0;\n");
            code.push_str("}\n\n");
        }
    }

    code
}

fn parse_shader_code(code: &str) -> usize {
    // Simulate shader parsing
    let mut function_count = 0;
    let mut struct_count = 0;

    for line in code.lines() {
        if line.trim().starts_with("fn ") {
            function_count += 1;
        } else if line.trim().starts_with("struct ") {
            struct_count += 1;
        }
    }

    function_count + struct_count
}

fn validate_shader_safety(code: &str) -> bool {
    // Simulate safety validation
    let unsafe_patterns = ["while", "for", "loop"];

    for pattern in &unsafe_patterns {
        if code.contains(pattern) {
            return false;
        }
    }

    true
}

criterion_group!(
    overlay_benches,
    benchmark_technical_indicators,
    benchmark_annotations,
    benchmark_custom_shaders
);
criterion_main!(overlay_benches);
