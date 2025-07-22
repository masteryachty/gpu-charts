//! Phase 3 Production Features benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, RwLock,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Telemetry system
struct TelemetrySystem {
    metrics: Arc<RwLock<HashMap<String, AtomicU64>>>,
    events: Arc<RwLock<Vec<TelemetryEvent>>>,
    enabled: AtomicBool,
}

#[derive(Clone)]
struct TelemetryEvent {
    timestamp: u64,
    event_type: String,
    properties: HashMap<String, String>,
}

impl TelemetrySystem {
    fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            events: Arc::new(RwLock::new(Vec::new())),
            enabled: AtomicBool::new(true),
        }
    }

    fn record_metric(&self, name: &str, value: u64) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        let metrics = self.metrics.read().unwrap();
        if let Some(metric) = metrics.get(name) {
            metric.fetch_add(value, Ordering::Relaxed);
        } else {
            drop(metrics);
            let mut metrics = self.metrics.write().unwrap();
            metrics
                .entry(name.to_string())
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(value, Ordering::Relaxed);
        }
    }

    fn record_event(&self, event_type: &str, properties: HashMap<String, String>) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        let event = TelemetryEvent {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            event_type: event_type.to_string(),
            properties,
        };

        let mut events = self.events.write().unwrap();
        events.push(event);

        // Simulate cleanup of old events
        if events.len() > 10000 {
            events.drain(0..5000);
        }
    }
}

/// Feature flag system
struct FeatureFlagSystem {
    flags: Arc<RwLock<HashMap<String, FeatureFlag>>>,
}

#[derive(Clone)]
struct FeatureFlag {
    enabled: bool,
    rollout_percentage: f32,
    user_overrides: HashMap<String, bool>,
}

impl FeatureFlagSystem {
    fn new() -> Self {
        let mut flags = HashMap::new();

        // Initialize some default flags
        flags.insert(
            "new_scatter_plot".to_string(),
            FeatureFlag {
                enabled: true,
                rollout_percentage: 100.0,
                user_overrides: HashMap::new(),
            },
        );

        flags.insert(
            "experimental_3d".to_string(),
            FeatureFlag {
                enabled: true,
                rollout_percentage: 50.0,
                user_overrides: HashMap::new(),
            },
        );

        flags.insert(
            "advanced_telemetry".to_string(),
            FeatureFlag {
                enabled: false,
                rollout_percentage: 0.0,
                user_overrides: HashMap::new(),
            },
        );

        Self {
            flags: Arc::new(RwLock::new(flags)),
        }
    }

    fn is_enabled(&self, flag_name: &str, user_id: Option<&str>) -> bool {
        let flags = self.flags.read().unwrap();

        if let Some(flag) = flags.get(flag_name) {
            // Check user override first
            if let Some(user_id) = user_id {
                if let Some(&override_value) = flag.user_overrides.get(user_id) {
                    return override_value;
                }
            }

            // Check if globally enabled
            if !flag.enabled {
                return false;
            }

            // Check rollout percentage
            if flag.rollout_percentage >= 100.0 {
                return true;
            }

            // Simple hash-based rollout
            if let Some(user_id) = user_id {
                let hash = user_id
                    .bytes()
                    .fold(0u32, |acc, b| acc.wrapping_add(b as u32));
                let threshold = (flag.rollout_percentage * 42949672.95) as u32; // max u32 / 100
                hash < threshold
            } else {
                false
            }
        } else {
            false
        }
    }
}

/// React bridge performance
struct ReactBridge {
    update_queue: Arc<RwLock<Vec<ChartUpdate>>>,
    props_cache: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

#[derive(Clone)]
struct ChartUpdate {
    update_type: UpdateType,
    data: Vec<f32>,
    timestamp: u64,
}

#[derive(Clone)]
enum UpdateType {
    DataUpdate,
    StyleUpdate,
    ViewportUpdate,
}

impl ReactBridge {
    fn new() -> Self {
        Self {
            update_queue: Arc::new(RwLock::new(Vec::new())),
            props_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn queue_update(&self, update: ChartUpdate) {
        let mut queue = self.update_queue.write().unwrap();
        queue.push(update);

        // Batch updates if queue gets too large
        if queue.len() > 100 {
            // In real implementation, this would trigger a batch update
            queue.clear();
        }
    }

    fn process_props(&self, props: &serde_json::Value) -> bool {
        let key = props.to_string();
        let mut cache = self.props_cache.write().unwrap();

        if cache.get(&key).map_or(true, |cached| cached != props) {
            cache.insert(key, props.clone());
            true // Props changed
        } else {
            false // Props unchanged
        }
    }
}

/// Benchmark telemetry overhead
fn benchmark_telemetry(c: &mut Criterion) {
    let mut group = c.benchmark_group("telemetry");

    let telemetry = TelemetrySystem::new();

    // Benchmark metric recording
    group.bench_function("record_metric", |b| {
        b.iter(|| {
            telemetry.record_metric("render_time", black_box(16));
        });
    });

    // Benchmark event recording
    group.bench_function("record_event", |b| {
        let mut props = HashMap::new();
        props.insert("chart_type".to_string(), "line".to_string());
        props.insert("point_count".to_string(), "100000".to_string());

        b.iter(|| {
            telemetry.record_event("chart_rendered", black_box(props.clone()));
        });
    });

    // Benchmark telemetry disabled overhead
    telemetry.enabled.store(false, Ordering::Relaxed);
    group.bench_function("record_metric_disabled", |b| {
        b.iter(|| {
            telemetry.record_metric("render_time", black_box(16));
        });
    });

    // Benchmark batch metric recording
    telemetry.enabled.store(true, Ordering::Relaxed);
    group.bench_function("record_metrics_batch", |b| {
        b.iter(|| {
            for i in 0..100 {
                telemetry.record_metric(&format!("metric_{}", i % 10), black_box(i as u64));
            }
        });
    });

    group.finish();
}

/// Benchmark feature flag system
fn benchmark_feature_flags(c: &mut Criterion) {
    let mut group = c.benchmark_group("feature_flags");

    let flags = FeatureFlagSystem::new();

    // Benchmark simple flag check
    group.bench_function("check_enabled_flag", |b| {
        b.iter(|| black_box(flags.is_enabled("new_scatter_plot", None)));
    });

    // Benchmark flag check with user
    group.bench_function("check_flag_with_user", |b| {
        b.iter(|| black_box(flags.is_enabled("experimental_3d", Some("user123"))));
    });

    // Benchmark disabled flag check (fast path)
    group.bench_function("check_disabled_flag", |b| {
        b.iter(|| black_box(flags.is_enabled("advanced_telemetry", Some("user123"))));
    });

    // Benchmark multiple flag checks
    group.bench_function("check_multiple_flags", |b| {
        b.iter(|| {
            let mut results = Vec::with_capacity(10);
            results.push(flags.is_enabled("new_scatter_plot", Some("user123")));
            results.push(flags.is_enabled("experimental_3d", Some("user123")));
            results.push(flags.is_enabled("advanced_telemetry", Some("user123")));
            results.push(flags.is_enabled("nonexistent_flag", Some("user123")));
            black_box(results)
        });
    });

    group.finish();
}

/// Benchmark React integration
fn benchmark_react_bridge(c: &mut Criterion) {
    let mut group = c.benchmark_group("react_bridge");

    let bridge = ReactBridge::new();

    // Benchmark update queueing
    group.bench_function("queue_update", |b| {
        let update = ChartUpdate {
            update_type: UpdateType::DataUpdate,
            data: vec![1.0; 1000],
            timestamp: 0,
        };

        b.iter(|| {
            bridge.queue_update(black_box(update.clone()));
        });
    });

    // Benchmark props processing
    group.bench_function("process_props", |b| {
        let props = serde_json::json!({
            "width": 1920,
            "height": 1080,
            "theme": "dark",
            "showGrid": true,
            "data": [1.0, 2.0, 3.0, 4.0, 5.0]
        });

        b.iter(|| black_box(bridge.process_props(&props)));
    });

    // Benchmark props caching effectiveness
    group.bench_function("process_props_cached", |b| {
        let props = serde_json::json!({
            "width": 1920,
            "height": 1080,
            "theme": "dark"
        });

        // Prime the cache
        bridge.process_props(&props);

        b.iter(|| black_box(bridge.process_props(&props)));
    });

    // Benchmark batch update processing
    group.bench_function("batch_updates", |b| {
        b.iter(|| {
            for i in 0..50 {
                let update = ChartUpdate {
                    update_type: match i % 3 {
                        0 => UpdateType::DataUpdate,
                        1 => UpdateType::StyleUpdate,
                        _ => UpdateType::ViewportUpdate,
                    },
                    data: vec![i as f32; 100],
                    timestamp: i as u64,
                };
                bridge.queue_update(update);
            }
        });
    });

    group.finish();
}

/// Benchmark CDN optimization features
fn benchmark_cdn_optimization(c: &mut Criterion) {
    let mut group = c.benchmark_group("cdn_optimization");

    // Benchmark cache key generation
    group.bench_function("cache_key_generation", |b| {
        b.iter(|| {
            let version = "1.2.3";
            let features = vec!["scatter", "heatmap", "3d"];
            let locale = "en-US";

            let key = format!("gpu-charts-{}-{}-{}", version, features.join("-"), locale);
            black_box(key)
        });
    });

    // Benchmark asset fingerprinting
    group.bench_function("asset_fingerprinting", |b| {
        let content = vec![0u8; 100_000]; // 100KB file

        b.iter(|| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::hash::Hash::hash(&content[..], &mut hasher);
            let hash = std::hash::Hasher::finish(&hasher);
            black_box(format!("chart-{:016x}.wasm", hash))
        });
    });

    // Benchmark version comparison
    group.bench_function("version_comparison", |b| {
        let current_version = "1.2.3";
        let required_version = "1.2.0";

        b.iter(|| {
            let current: Vec<u32> = current_version
                .split('.')
                .map(|s| s.parse().unwrap())
                .collect();
            let required: Vec<u32> = required_version
                .split('.')
                .map(|s| s.parse().unwrap())
                .collect();

            let compatible = current[0] == required[0]
                && (current[1] > required[1]
                    || (current[1] == required[1] && current[2] >= required[2]));
            black_box(compatible)
        });
    });

    group.finish();
}

criterion_group!(
    production_benches,
    benchmark_telemetry,
    benchmark_feature_flags,
    benchmark_react_bridge,
    benchmark_cdn_optimization
);
criterion_main!(production_benches);
