# WASM Compatibility Solutions: Concrete Implementation Guide

## Overview

This document provides battle-tested solutions for the most challenging WASM compatibility issues when integrating Phase 2 & 3 components.

## 1. Network Operations: From Hyper to Fetch API

### Problem: Hyper/Reqwest Don't Work in WASM

#### ❌ Current Phase 2 Code:
```rust
// This won't compile to WASM
use hyper::{Client, Body};
use hyper_tls::HttpsConnector;

pub async fn fetch_data(url: &str) -> Result<Vec<u8>> {
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, Body>(https);
    let resp = client.get(url.parse()?).await?;
    // ...
}
```

#### ✅ WASM-Compatible Solution:
```rust
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{Request, RequestInit, Response, Headers};
        use js_sys::Uint8Array;
        
        pub async fn fetch_data(url: &str) -> Result<Vec<u8>, JsValue> {
            let window = web_sys::window().unwrap();
            
            // Create request with headers
            let mut opts = RequestInit::new();
            opts.method("GET");
            
            let headers = Headers::new()?;
            headers.set("Accept", "application/octet-stream")?;
            opts.headers(&headers);
            
            let request = Request::new_with_str_and_init(url, &opts)?;
            
            // Make request
            let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
            let resp: Response = resp_value.dyn_into()?;
            
            // Check status
            if !resp.ok() {
                return Err(JsValue::from_str(&format!("HTTP {}", resp.status())));
            }
            
            // Get body as array buffer
            let array_buffer = JsFuture::from(resp.array_buffer()?).await?;
            let uint8_array = Uint8Array::new(&array_buffer);
            
            // Convert to Vec<u8>
            let mut result = vec![0u8; uint8_array.length() as usize];
            uint8_array.copy_to(&mut result);
            
            Ok(result)
        }
    } else {
        // Native implementation using hyper
        use hyper::{Client, Body};
        use hyper_tls::HttpsConnector;
        use bytes::Bytes;
        
        pub async fn fetch_data(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let https = HttpsConnector::new();
            let client = Client::builder().build::<_, Body>(https);
            let resp = client.get(url.parse()?).await?;
            let bytes = hyper::body::to_bytes(resp.into_body()).await?;
            Ok(bytes.to_vec())
        }
    }
}
```

### Streaming Large Datasets

```rust
#[cfg(target_arch = "wasm32")]
pub async fn fetch_streaming(url: &str, on_chunk: impl Fn(&[u8])) -> Result<(), JsValue> {
    use web_sys::ReadableStream;
    use wasm_streams::ReadableStream as WasmStream;
    
    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_str(url)).await?;
    let resp: Response = resp_value.dyn_into()?;
    
    let body = resp.body().ok_or("No body")?;
    let stream = WasmStream::from_raw(body);
    let mut reader = stream.get_reader();
    
    loop {
        match reader.read().await? {
            Some(chunk) => {
                let array = Uint8Array::new(&chunk);
                let mut bytes = vec![0u8; array.length() as usize];
                array.copy_to(&mut bytes);
                on_chunk(&bytes);
            }
            None => break,
        }
    }
    
    Ok(())
}
```

## 2. WebSocket: From Tokio-Tungstenite to Browser API

### Problem: Native WebSocket Libraries Don't Work

#### ❌ Current Phase 2 Code:
```rust
use tokio_tungstenite::{connect_async, WebSocketStream};

pub async fn connect_ws(url: &str) -> Result<WebSocketStream> {
    let (ws_stream, _) = connect_async(url).await?;
    // ...
}
```

#### ✅ WASM-Compatible Solution:
```rust
#[cfg(target_arch = "wasm32")]
pub struct WasmWebSocket {
    ws: web_sys::WebSocket,
    rx: mpsc::UnboundedReceiver<WsMessage>,
    _closures: Vec<Closure<dyn FnMut(JsValue)>>,
}

#[cfg(target_arch = "wasm32")]
impl WasmWebSocket {
    pub fn connect(url: &str) -> Result<Self, JsValue> {
        use futures::channel::mpsc;
        
        let ws = web_sys::WebSocket::new(url)?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
        
        let (tx, rx) = mpsc::unbounded();
        let mut closures = Vec::new();
        
        // On message
        {
            let tx = tx.clone();
            let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                    let array = js_sys::Uint8Array::new(&abuf);
                    let mut vec = vec![0u8; array.length() as usize];
                    array.copy_to(&mut vec);
                    let _ = tx.unbounded_send(WsMessage::Binary(vec));
                } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                    let _ = tx.unbounded_send(WsMessage::Text(txt.as_string().unwrap()));
                }
            }) as Box<dyn FnMut(_)>);
            
            ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            closures.push(onmessage);
        }
        
        // On error
        {
            let tx = tx.clone();
            let onerror = Closure::wrap(Box::new(move |_| {
                let _ = tx.unbounded_send(WsMessage::Error("WebSocket error".into()));
            }) as Box<dyn FnMut(_)>);
            
            ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            closures.push(onerror);
        }
        
        // On close
        {
            let tx = tx.clone();
            let onclose = Closure::wrap(Box::new(move |_| {
                let _ = tx.unbounded_send(WsMessage::Close);
            }) as Box<dyn FnMut(_)>);
            
            ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
            closures.push(onclose);
        }
        
        Ok(WasmWebSocket {
            ws,
            rx,
            _closures: closures,
        })
    }
    
    pub async fn recv(&mut self) -> Option<WsMessage> {
        self.rx.next().await
    }
    
    pub fn send(&self, data: &[u8]) -> Result<(), JsValue> {
        self.ws.send_with_u8_array(data)
    }
}
```

## 3. File Operations: From FS/MMap to Browser Storage

### Problem: No File System Access in Browser

#### ❌ Current Phase 2 Code:
```rust
use memmap2::MmapOptions;
use std::fs::File;

pub fn load_cached_data(path: &str) -> Result<Mmap> {
    let file = File::open(path)?;
    unsafe { MmapOptions::new().map(&file) }
}
```

#### ✅ WASM-Compatible Solutions:

### Option 1: IndexedDB for Large Data
```rust
#[cfg(target_arch = "wasm32")]
pub mod storage {
    use indexed_db_futures::prelude::*;
    use wasm_bindgen::JsValue;
    
    pub struct IndexedDBCache {
        db: IdbDatabase,
    }
    
    impl IndexedDBCache {
        pub async fn new() -> Result<Self, JsValue> {
            let mut db_req = IdbDatabase::open("gpu_charts_cache")?;
            db_req.set_on_upgrade_needed(Some(|evt: &IdbVersionChangeEvent| {
                if !evt.db().object_store_names().contains(&"data") {
                    evt.db().create_object_store("data")?;
                }
                Ok(())
            }));
            
            let db = db_req.await?;
            Ok(IndexedDBCache { db })
        }
        
        pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, JsValue> {
            let tx = self.db.transaction(&["data"], IdbTransactionMode::Readonly)?;
            let store = tx.object_store("data")?;
            
            match store.get(&JsValue::from_str(key))?.await? {
                Some(value) => {
                    let array = js_sys::Uint8Array::new(&value);
                    let mut vec = vec![0u8; array.length() as usize];
                    array.copy_to(&mut vec);
                    Ok(Some(vec))
                }
                None => Ok(None),
            }
        }
        
        pub async fn put(&self, key: &str, data: &[u8]) -> Result<(), JsValue> {
            let tx = self.db.transaction(&["data"], IdbTransactionMode::Readwrite)?;
            let store = tx.object_store("data")?;
            
            let array = js_sys::Uint8Array::from(data);
            store.put(&array.into(), &JsValue::from_str(key))?.await?;
            tx.await?;
            
            Ok(())
        }
    }
}
```

### Option 2: Local Storage for Small Config
```rust
#[cfg(target_arch = "wasm32")]
pub fn save_config(config: &Config) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let storage = window.local_storage()?.unwrap();
    let json = serde_json::to_string(config).map_err(|e| e.to_string())?;
    storage.set_item("gpu_charts_config", &json)?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn load_config() -> Result<Option<Config>, JsValue> {
    let window = web_sys::window().unwrap();
    let storage = window.local_storage()?.unwrap();
    
    match storage.get_item("gpu_charts_config")? {
        Some(json) => {
            let config = serde_json::from_str(&json)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            Ok(Some(config))
        }
        None => Ok(None),
    }
}
```

## 4. Threading: From std::thread to Web Workers

### Problem: No OS Threads in WASM

#### ❌ Current Phase 2 Code:
```rust
use std::thread;

pub fn parallel_compute(data: Vec<f32>) -> Vec<f32> {
    let chunks: Vec<_> = data.chunks(1000).collect();
    let handles: Vec<_> = chunks.into_iter()
        .map(|chunk| {
            thread::spawn(move || process_chunk(chunk))
        })
        .collect();
    // ...
}
```

#### ✅ WASM-Compatible Solutions:

### Option 1: Web Workers for Heavy Computation
```rust
#[cfg(target_arch = "wasm32")]
pub struct WorkerPool {
    workers: Vec<web_sys::Worker>,
    tasks: mpsc::UnboundedSender<Task>,
}

#[cfg(target_arch = "wasm32")]
impl WorkerPool {
    pub fn new(worker_script: &str, num_workers: usize) -> Result<Self, JsValue> {
        let mut workers = Vec::new();
        let (tx, mut rx) = mpsc::unbounded();
        
        for _ in 0..num_workers {
            let worker = web_sys::Worker::new(worker_script)?;
            
            let tx_clone = tx.clone();
            let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                // Handle worker results
            }) as Box<dyn FnMut(_)>);
            
            worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget();
            
            workers.push(worker);
        }
        
        Ok(WorkerPool { workers, tasks: tx })
    }
    
    pub fn execute(&self, data: Vec<f32>) -> impl Future<Output = Vec<f32>> {
        // Distribute work to workers
        async move {
            // Implementation
            data // Placeholder
        }
    }
}
```

### Option 2: WASM Threads (if available)
```rust
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
pub fn parallel_compute_wasm(data: Vec<f32>) -> Vec<f32> {
    use wasm_bindgen::prelude::*;
    use web_sys::WorkerGlobalScope;
    
    // Only if SharedArrayBuffer is available
    // Requires special headers: Cross-Origin-Embedder-Policy: require-corp
    // and Cross-Origin-Opener-Policy: same-origin
    
    // Implementation using shared memory
    data // Placeholder
}
```

## 5. SIMD: From Native to WASM SIMD

### Problem: Different SIMD APIs

#### ✅ Cross-Platform SIMD Solution:
```rust
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))] {
        use core::arch::wasm32::*;
        
        pub fn simd_sum(data: &[f32]) -> f32 {
            let mut sum = f32x4_splat(0.0);
            let chunks = data.chunks_exact(4);
            let remainder = chunks.remainder();
            
            for chunk in chunks {
                let v = v128_load(chunk.as_ptr() as *const v128);
                sum = f32x4_add(sum, v);
            }
            
            let mut result = f32x4_extract_lane::<0>(sum)
                + f32x4_extract_lane::<1>(sum)
                + f32x4_extract_lane::<2>(sum)
                + f32x4_extract_lane::<3>(sum);
            
            for &x in remainder {
                result += x;
            }
            
            result
        }
    } else if #[cfg(target_arch = "x86_64")] {
        use std::arch::x86_64::*;
        
        pub fn simd_sum(data: &[f32]) -> f32 {
            unsafe {
                // x86_64 SIMD implementation
                data.iter().sum() // Placeholder
            }
        }
    } else {
        pub fn simd_sum(data: &[f32]) -> f32 {
            data.iter().sum()
        }
    }
}
```

## 6. Performance Monitoring

### WASM-Specific Performance Tools:
```rust
#[cfg(target_arch = "wasm32")]
pub struct PerformanceMonitor {
    performance: web_sys::Performance,
}

#[cfg(target_arch = "wasm32")]
impl PerformanceMonitor {
    pub fn new() -> Self {
        let window = web_sys::window().unwrap();
        let performance = window.performance().unwrap();
        PerformanceMonitor { performance }
    }
    
    pub fn mark(&self, name: &str) {
        self.performance.mark(name).ok();
    }
    
    pub fn measure(&self, name: &str, start: &str, end: &str) -> f64 {
        self.performance.measure_with_start_mark_and_end_mark(name, start, end).ok();
        
        if let Ok(entries) = self.performance.get_entries_by_name(name) {
            if let Some(entry) = entries.get(0) {
                if let Ok(measure) = entry.dyn_into::<web_sys::PerformanceMeasure>() {
                    return measure.duration();
                }
            }
        }
        
        0.0
    }
    
    pub fn memory_usage(&self) -> Option<MemoryInfo> {
        // Only available in Chrome with --enable-precise-memory-info
        js_sys::Reflect::get(&self.performance, &"memory".into())
            .ok()
            .and_then(|memory| {
                let used = js_sys::Reflect::get(&memory, &"usedJSHeapSize".into())
                    .ok()?
                    .as_f64()?;
                let total = js_sys::Reflect::get(&memory, &"totalJSHeapSize".into())
                    .ok()?
                    .as_f64()?;
                
                Some(MemoryInfo {
                    used: used as usize,
                    total: total as usize,
                })
            })
    }
}
```

## 7. Error Handling

### Unified Error Type:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChartError {
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Render error: {0}")]
    Render(String),
    
    #[error("Data error: {0}")]
    Data(String),
    
    #[cfg(target_arch = "wasm32")]
    #[error("JavaScript error: {0}")]
    JsError(String),
    
    #[cfg(not(target_arch = "wasm32"))]
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

// Convert for WASM
#[cfg(target_arch = "wasm32")]
impl From<JsValue> for ChartError {
    fn from(err: JsValue) -> Self {
        ChartError::JsError(format!("{:?}", err))
    }
}

// Convert to JsValue for WASM bindings
#[cfg(target_arch = "wasm32")]
impl From<ChartError> for JsValue {
    fn from(err: ChartError) -> Self {
        JsValue::from_str(&err.to_string())
    }
}
```

## 8. Build Configuration

### Cargo.toml for Cross-Platform:
```toml
[package]
name = "gpu-charts-unified"

[features]
default = []
wasm = ["wasm-bindgen", "web-sys", "js-sys", "wasm-streams", "indexed-db-futures"]
native = ["tokio/full", "hyper", "hyper-tls", "memmap2"]
simd = []

[dependencies]
# Always included
cfg-if = "1.0"
futures = "0.3"
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"

# WASM dependencies
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
    "Window", "Document", "Element", "HtmlCanvasElement",
    "Request", "RequestInit", "Response", "Headers",
    "WebSocket", "MessageEvent", "ErrorEvent", "CloseEvent",
    "Storage", "Performance", "PerformanceMeasure",
    "Worker", "WorkerGlobalScope", "ReadableStream",
    "console"
]}
js-sys = "0.3"
wasm-streams = "0.4"
indexed-db-futures = "0.4"
console_error_panic_hook = "0.1"

# Native dependencies
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio = { version = "1", features = ["full"] }
hyper = { version = "0.14", features = ["client", "http2"] }
hyper-tls = "0.5"
memmap2 = "0.9"

# Build script for feature detection
[build-dependencies]
wasm-bindgen = "0.2"
```

## Testing Both Platforms

### Test Harness:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_native_fetch() {
        let data = fetch_data("https://example.com/data").await.unwrap();
        assert!(!data.is_empty());
    }
    
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;
    
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn test_wasm_fetch() {
        let data = fetch_data("/test-data").await.unwrap();
        assert!(!data.is_empty());
    }
}
```

## Key Takeaways

1. **Always use cfg_if** for platform-specific code
2. **Design APIs to be async** - works for both platforms
3. **Use web-sys for browser APIs** - well-maintained and complete
4. **Test on both platforms** - behavior can differ
5. **Handle errors gracefully** - WASM errors are different
6. **Monitor performance** - WASM has unique characteristics
7. **Document platform differences** - helps other developers

This guide provides concrete, working solutions for the most common WASM compatibility challenges you'll face when integrating Phase 2 & 3 components.