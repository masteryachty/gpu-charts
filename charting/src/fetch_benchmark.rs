use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;
use js_sys::{ArrayBuffer, Uint8Array};

// Import the wasm-fetch crate from the workspace
use wasm_fetch::FetchClient;

#[wasm_bindgen]
pub struct FetchBenchmark {
    test_url: String,
}

#[wasm_bindgen]
impl FetchBenchmark {
    #[wasm_bindgen(constructor)]
    pub fn new(test_url: String) -> Self {
        Self { test_url }
    }

    /// Run the old fetch method N times and return average time in milliseconds
    #[wasm_bindgen]
    pub async fn benchmark_old_method(&self, iterations: u32) -> Result<f64, JsValue> {
        let performance = web_sys::window()
            .ok_or("No window")?
            .performance()
            .ok_or("No performance API")?;
        
        let mut total_time = 0.0;
        
        for _ in 0..iterations {
            let start = performance.now();
            
            // Old method
            let window = web_sys::window().ok_or("No window")?;
            let resp = JsFuture::from(window.fetch_with_str(&self.test_url))
                .await
                .map_err(|e| JsValue::from_str(&format!("Fetch failed: {e:?}")))?;
            
            let resp: Response = resp.unchecked_into();
            let array_buffer: ArrayBuffer = JsFuture::from(resp.array_buffer()?)
                .await
                .map(|v| v.unchecked_into::<ArrayBuffer>())
                .map_err(|e| JsValue::from_str(&format!("ArrayBuffer conversion failed: {e:?}")))?;
            
            let end = performance.now();
            total_time += end - start;
            
            // Log the buffer size to ensure we're getting data
            if iterations == 1 {
                log::info!("Old method - ArrayBuffer size: {} bytes", array_buffer.byte_length());
            }
        }
        
        Ok(total_time / iterations as f64)
    }
    
    /// Run the new fetch method N times and return average time in milliseconds
    #[wasm_bindgen]
    pub async fn benchmark_new_method(&self, iterations: u32) -> Result<f64, JsValue> {
        let performance = web_sys::window()
            .ok_or("No window")?
            .performance()
            .ok_or("No performance API")?;
        
        let mut total_time = 0.0;
        
        for _ in 0..iterations {
            let start = performance.now();
            
            // New method
            let client = FetchClient::new();
            let binary_data = client.fetch_binary(&self.test_url)
                .await
                .map_err(|e| JsValue::from_str(&format!("Fetch failed: {:?}", e)))?;
            
            // Convert Vec<u8> to ArrayBuffer
            let uint8_array = Uint8Array::new_with_length(binary_data.len() as u32);
            uint8_array.copy_from(&binary_data);
            let array_buffer = uint8_array.buffer();
            
            let end = performance.now();
            total_time += end - start;
            
            // Log the buffer size to ensure we're getting data
            if iterations == 1 {
                log::info!("New method - ArrayBuffer size: {} bytes", array_buffer.byte_length());
            }
        }
        
        Ok(total_time / iterations as f64)
    }
    
    /// Run a comprehensive benchmark comparing both methods
    #[wasm_bindgen]
    pub async fn run_comparison(&self, iterations: u32) -> String {
        log::info!("Starting fetch benchmark with {} iterations", iterations);
        
        // Warm up - run each method once to avoid cold start bias
        let _ = self.benchmark_old_method(1).await;
        let _ = self.benchmark_new_method(1).await;
        
        // Run actual benchmarks
        let old_time = match self.benchmark_old_method(iterations).await {
            Ok(time) => time,
            Err(e) => return format!("Old method error: {:?}", e),
        };
        
        let new_time = match self.benchmark_new_method(iterations).await {
            Ok(time) => time,
            Err(e) => return format!("New method error: {:?}", e),
        };
        
        let difference = old_time - new_time;
        let percent_change = ((old_time - new_time) / old_time) * 100.0;
        
        format!(
            "Benchmark Results ({} iterations):\n\
             Old method average: {:.2} ms\n\
             New method average: {:.2} ms\n\
             Difference: {:.2} ms\n\
             Performance change: {:.1}% {}\n\
             \n\
             The {} method is faster.",
            iterations,
            old_time,
            new_time,
            difference.abs(),
            percent_change.abs(),
            if percent_change > 0.0 { "faster" } else { "slower" },
            if new_time < old_time { "new" } else { "old" }
        )
    }
    
    /// Test memory usage by fetching data multiple times
    #[wasm_bindgen]
    pub async fn benchmark_memory_usage(&self, fetches: u32) -> Result<String, JsValue> {
        let performance = web_sys::window()
            .ok_or("No window")?
            .performance()
            .ok_or("No performance API")?;
        
        // Get initial memory if available
        let initial_memory = Self::get_memory_usage();
        
        // Test old method memory usage
        let old_start = performance.now();
        for _ in 0..fetches {
            let window = web_sys::window().ok_or("No window")?;
            let resp = JsFuture::from(window.fetch_with_str(&self.test_url))
                .await
                .map_err(|e| JsValue::from_str(&format!("Fetch failed: {e:?}")))?;
            
            let resp: Response = resp.unchecked_into();
            let _array_buffer: ArrayBuffer = JsFuture::from(resp.array_buffer()?)
                .await
                .map(|v| v.unchecked_into::<ArrayBuffer>())
                .map_err(|e| JsValue::from_str(&format!("ArrayBuffer conversion failed: {e:?}")))?;
        }
        let old_end = performance.now();
        let old_memory = Self::get_memory_usage();
        
        // Force garbage collection if available
        Self::try_gc();
        
        // Wait a bit
        let sleep_promise = js_sys::Promise::new(&mut |resolve, _| {
            let closure = Closure::once_into_js(move || {
                resolve.call0(&JsValue::undefined()).unwrap();
            });
            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    1000
                )
                .unwrap();
        });
        let _ = JsFuture::from(sleep_promise).await;
        
        let mid_memory = Self::get_memory_usage();
        
        // Test new method memory usage
        let new_start = performance.now();
        for _ in 0..fetches {
            let client = FetchClient::new();
            let binary_data = client.fetch_binary(&self.test_url)
                .await
                .map_err(|e| JsValue::from_str(&format!("Fetch failed: {:?}", e)))?;
            
            let uint8_array = Uint8Array::new_with_length(binary_data.len() as u32);
            uint8_array.copy_from(&binary_data);
            let _array_buffer = uint8_array.buffer();
        }
        let new_end = performance.now();
        let new_memory = Self::get_memory_usage();
        
        Ok(format!(
            "Memory Usage Benchmark ({} fetches):\n\
             Initial memory: {}\n\
             After old method: {} (delta: {})\n\
             After GC: {} (delta: {})\n\
             After new method: {} (delta: {})\n\
             \n\
             Time taken:\n\
             Old method: {:.2} ms total ({:.2} ms per fetch)\n\
             New method: {:.2} ms total ({:.2} ms per fetch)",
            fetches,
            Self::format_memory(initial_memory),
            Self::format_memory(old_memory),
            Self::format_memory(old_memory.saturating_sub(initial_memory)),
            Self::format_memory(mid_memory),
            Self::format_memory(mid_memory.saturating_sub(initial_memory)),
            Self::format_memory(new_memory),
            Self::format_memory(new_memory.saturating_sub(mid_memory)),
            old_end - old_start,
            (old_end - old_start) / fetches as f64,
            new_end - new_start,
            (new_end - new_start) / fetches as f64
        ))
    }
    
    fn get_memory_usage() -> u32 {
        if let Some(window) = web_sys::window() {
            if let Ok(performance) = js_sys::Reflect::get(&window, &JsValue::from_str("performance")) {
                if let Ok(memory) = js_sys::Reflect::get(&performance, &JsValue::from_str("memory")) {
                    if let Ok(used) = js_sys::Reflect::get(&memory, &JsValue::from_str("usedJSHeapSize")) {
                        return used.as_f64().unwrap_or(0.0) as u32;
                    }
                }
            }
        }
        0
    }
    
    fn format_memory(bytes: u32) -> String {
        if bytes == 0 {
            "N/A".to_string()
        } else if bytes < 1024 * 1024 {
            format!("{:.2} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    }
    
    fn try_gc() {
        if let Some(window) = web_sys::window() {
            if let Ok(gc) = js_sys::Reflect::get(&window, &JsValue::from_str("gc")) {
                if gc.is_function() {
                    let _ = js_sys::Reflect::apply(
                        gc.unchecked_ref::<js_sys::Function>(),
                        &JsValue::undefined(),
                        &js_sys::Array::new()
                    );
                }
            }
        }
    }
}