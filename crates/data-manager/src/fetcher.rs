//! High-performance HTTP/2 data fetcher with streaming support

use gpu_charts_shared::{Error, Result};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

/// Fetches binary data from the server with maximum performance
pub struct DataFetcher {
    base_url: String,
}

impl DataFetcher {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    /// Fetch data with HTTP/2 streaming
    pub async fn fetch_binary(&self, url: &str) -> Result<Vec<u8>> {
        let window = web_sys::window()
            .ok_or_else(|| Error::NetworkError("No window object available".to_string()))?;

        let mut opts = RequestInit::new();
        opts.method("GET");

        let request = Request::new_with_str_and_init(url, &opts)
            .map_err(|_| Error::NetworkError("Failed to create request".to_string()))?;

        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|_| Error::NetworkError("Fetch failed".to_string()))?;

        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| Error::NetworkError("Invalid response type".to_string()))?;

        if !resp.ok() {
            return Err(Error::NetworkError(format!("HTTP {}", resp.status())));
        }

        // Get array buffer
        let buffer = JsFuture::from(
            resp.array_buffer()
                .map_err(|_| Error::NetworkError("Failed to get array buffer".to_string()))?,
        )
        .await
        .map_err(|_| Error::NetworkError("Failed to read array buffer".to_string()))?;

        // Convert to Vec<u8>
        let array = js_sys::Uint8Array::new(&buffer);
        let mut vec = vec![0u8; array.length() as usize];
        array.copy_to(&mut vec);

        Ok(vec)
    }

    /// Fetch with progress callback for large datasets
    pub async fn fetch_with_progress<F>(&self, url: &str, mut on_progress: F) -> Result<Vec<u8>>
    where
        F: FnMut(u64, u64) + 'static,
    {
        // TODO: Implement streaming with ReadableStream API
        // For now, fallback to regular fetch
        self.fetch_binary(url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would need to run in a WASM environment
    // For now, they're placeholders
}
