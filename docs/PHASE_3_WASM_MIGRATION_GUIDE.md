# Phase 3 WASM Migration Quick Reference

## Dependency Replacements

### Network Operations

❌ **Don't Use**:
```rust
use hyper::Client;
use tokio::net::TcpStream;
use reqwest::Client;
```

✅ **Use Instead**:
```rust
use web_sys::{Request, RequestInit, Response};
use wasm_bindgen_futures::JsFuture;

pub async fn fetch_data(url: &str) -> Result<Vec<u8>> {
    let window = web_sys::window().unwrap();
    let request = Request::new_with_str(url)?;
    let response: Response = JsFuture::from(
        window.fetch_with_request(&request)
    ).await?.dyn_into()?;
    // Convert response to bytes
}
```

### WebSocket

❌ **Don't Use**:
```rust
use tokio_tungstenite::connect_async;
```

✅ **Use Instead**:
```rust
use web_sys::WebSocket;

pub fn create_websocket(url: &str) -> Result<WebSocket> {
    let ws = WebSocket::new(url)?;
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
    // Set up callbacks
    Ok(ws)
}
```

### File Operations

❌ **Don't Use**:
```rust
use std::fs::File;
use memmap2::Mmap;
```

✅ **Use Instead**:
```rust
// For config storage
use web_sys::Storage;
let storage = window.local_storage()?.unwrap();
storage.set_item("config", &json)?;

// For large data
use indexed_db_futures::IdbDatabase;
```

### Threading

❌ **Don't Use**:
```rust
use tokio::spawn;
use std::thread;
```

✅ **Use Instead**:
```rust
// Web Workers for heavy computation
use web_sys::Worker;

// Or use wasm-bindgen-futures for async
use wasm_bindgen_futures::spawn_local;
```

### Compression

❌ **Don't Use**:
```rust
use zstd;  // Has C dependencies
```

✅ **Use Instead**:
```rust
use flate2;  // Pure Rust
use brotli;  // Pure Rust
// Or use browser's DecompressionStream API
```

## Cargo.toml Configuration

### For WASM-Compatible Crate

```toml
[dependencies]
# WASM essentials
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
    "Window", "Document", "Element", "HtmlCanvasElement",
    "Request", "RequestInit", "Response", "Headers",
    "WebSocket", "MessageEvent", "Storage",
    "Performance", "console"
]}
js-sys = "0.3"

# Avoid these in WASM builds
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio = { version = "1.0", features = ["full"] }
hyper = "0.14"
reqwest = "0.11"

[features]
default = []
native = ["tokio", "hyper", "reqwest"]
wasm = ["wasm-bindgen", "web-sys", "js-sys"]
```

## Code Patterns

### Conditional Compilation

```rust
#[cfg(target_arch = "wasm32")]
pub async fn load_data(url: &str) -> Result<Vec<u8>> {
    // WASM implementation using fetch()
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn load_data(url: &str) -> Result<Vec<u8>> {
    // Native implementation using reqwest
}
```

### Error Handling

```rust
// Define unified error type
#[derive(Debug)]
pub enum DataError {
    #[cfg(target_arch = "wasm32")]
    JsError(wasm_bindgen::JsValue),
    
    #[cfg(not(target_arch = "wasm32"))]
    IoError(std::io::Error),
    
    ParseError(String),
}
```

### Performance Monitoring

```rust
#[cfg(target_arch = "wasm32")]
pub fn measure_performance<F: FnOnce()>(name: &str, f: F) {
    let performance = web_sys::window()
        .unwrap()
        .performance()
        .unwrap();
    
    let start = performance.now();
    f();
    let end = performance.now();
    
    web_sys::console::log_1(
        &format!("{}: {}ms", name, end - start).into()
    );
}
```

## Build Commands

```bash
# Development build
wasm-pack build --dev --target web --out-dir ../../web/pkg

# Production build with optimization
wasm-pack build --release --target web --out-dir ../../web/pkg
wasm-opt -O4 -o optimized.wasm ../../web/pkg/module_bg.wasm

# With specific features
wasm-pack build --features wasm --no-default-features
```

## Common Pitfalls

1. **Forgetting to handle async differently**
   - WASM needs `wasm-bindgen-futures`
   - Can't use `tokio::main`

2. **Using blocking operations**
   - No `std::thread::sleep`
   - Use `js_sys::Promise` with timeout

3. **Assuming file access**
   - No `std::fs`
   - Use fetch() or IndexedDB

4. **Memory leaks with closures**
   - Always call `closure.forget()` for JS callbacks
   - Or use `Closure::once()`

5. **Large WASM files**
   - Enable LTO in release builds
   - Use `wee_alloc` for smaller allocator
   - Split into multiple modules

## Testing WASM Code

```rust
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
async fn test_fetch_data() {
    let data = fetch_data("/test").await;
    assert!(data.is_ok());
}
```

Run with:
```bash
wasm-pack test --chrome --headless
```

This guide should help with the actual implementation of migrating Phase 3 components to WASM.