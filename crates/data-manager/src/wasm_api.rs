//! WASM API for the data manager
//!
//! This module provides the JavaScript-facing API that maintains
//! zero data boundary crossings by using handle-based references.

use gpu_charts_shared::{DataHandle, DataRequest};
use wasm_bindgen::prelude::*;

/// WASM-accessible data manager wrapper
///
/// Note: The actual DataManager uses Arc<wgpu::Device> which cannot be directly
/// passed through wasm_bindgen. In production, the wasm-bridge crate will handle
/// the coordination between data-manager and renderer with proper device sharing.
#[wasm_bindgen]
pub struct WasmDataManager {
    // For now, we just expose the data fetching capabilities
    // The actual GPU buffer management will be handled internally
    _base_url: String,
}

#[wasm_bindgen]
impl WasmDataManager {
    /// Create a new data manager
    #[wasm_bindgen(constructor)]
    pub fn new(base_url: String) -> Self {
        console_error_panic_hook::set_once();

        WasmDataManager {
            _base_url: base_url,
        }
    }

    /// Fetch data and return a handle
    #[wasm_bindgen]
    pub async fn fetch_data(&mut self, request_json: &str) -> Result<String, JsValue> {
        // In a real implementation, this would coordinate with the actual DataManager
        // For now, return a mock response
        let request: DataRequest = serde_json::from_str(request_json)
            .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

        let handle = DataHandle {
            id: uuid::Uuid::new_v4(),
            metadata: gpu_charts_shared::DataMetadata {
                symbol: request.symbol,
                time_range: request.time_range,
                columns: request.columns,
                row_count: 0,
                byte_size: 0,
                creation_time: js_sys::Date::now() as u64,
            },
        };

        Ok(serde_json::to_string(&handle).unwrap())
    }

    /// Release a data handle
    #[wasm_bindgen]
    pub fn release_handle(&mut self, _handle_id: &str) {
        // TODO: Implement handle release
        log::info!("Released handle: {}", _handle_id);
    }

    /// Get cache and memory statistics
    #[wasm_bindgen]
    pub fn get_stats(&self) -> String {
        serde_json::json!({
            "cache": {
                "entries": 0,
                "total_size_mb": 0.0,
                "hit_rate": 0.0,
            },
            "buffer_pool": {
                "allocated_mb": 0.0,
            }
        })
        .to_string()
    }

    /// Prefetch data for anticipated user actions
    #[wasm_bindgen]
    pub async fn prefetch(&mut self, request_json: &str) -> Result<(), JsValue> {
        // Parse request to validate
        let _request: DataRequest = serde_json::from_str(request_json)
            .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

        // TODO: Implement actual prefetching
        log::info!("Prefetching data");

        Ok(())
    }

    /// Clear the cache
    #[wasm_bindgen]
    pub fn clear_cache(&mut self) {
        log::info!("Cache cleared");
    }
}

/// Get GPU buffer information from a data handle
#[wasm_bindgen]
pub fn get_gpu_buffer_info(handle_json: &str) -> Result<String, JsValue> {
    let handle: DataHandle = serde_json::from_str(handle_json)
        .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

    // In a real implementation, this would return actual GPU buffer references
    // that can be passed to the renderer without data copies
    Ok(serde_json::json!({
        "id": handle.id.to_string(),
        "buffer_count": handle.metadata.columns.len(),
        "total_size": handle.metadata.byte_size,
    })
    .to_string())
}
