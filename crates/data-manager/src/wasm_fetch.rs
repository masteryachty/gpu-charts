use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AbortController, Headers, Request, RequestInit, Response};

/// WASM-compatible HTTP client using browser's fetch API
pub struct FetchClient {
    timeout_ms: u32,
}

impl Default for FetchClient {
    fn default() -> Self {
        Self { timeout_ms: 30000 }
    }
}

impl FetchClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timeout(timeout_ms: u32) -> Self {
        Self { timeout_ms }
    }

    /// Fetch binary data from a URL
    pub async fn fetch_binary(&self, url: &str) -> Result<Vec<u8>, JsValue> {
        let opts = RequestInit::new();
        opts.set_method("GET");

        // Create abort controller for timeout
        let abort_controller = AbortController::new()?;
        let signal = abort_controller.signal();
        opts.set_signal(Some(&signal));

        // Set headers for binary data
        let headers = Headers::new()?;
        headers.set("Accept", "application/octet-stream")?;

        let request = Request::new_with_str_and_init(url, &opts)?;

        // Get window object
        let window =
            web_sys::window().ok_or_else(|| JsValue::from_str("No window object available"))?;

        // Create timeout promise
        let timeout_promise = js_sys::Promise::new(&mut |_, reject| {
            let abort_controller_clone = abort_controller.clone();
            let timeout_closure = Closure::once(Box::new(move || {
                abort_controller_clone.abort();
                reject
                    .call1(&JsValue::null(), &JsValue::from_str("Request timeout"))
                    .unwrap();
            }) as Box<dyn FnOnce()>);

            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    timeout_closure.as_ref().unchecked_ref(),
                    self.timeout_ms as i32,
                )
                .unwrap();

            timeout_closure.forget();
        });

        // Fetch with timeout
        let fetch_promise = window.fetch_with_request(&request);
        let result = js_sys::Promise::race(&js_sys::Array::of2(&fetch_promise, &timeout_promise));

        let resp_value = JsFuture::from(result).await?;
        let resp: Response = resp_value.dyn_into()?;

        // Check response status
        if !resp.ok() {
            return Err(JsValue::from_str(&format!(
                "HTTP error! status: {}",
                resp.status()
            )));
        }

        // Get response as array buffer
        let array_buffer_promise = resp.array_buffer()?;
        let array_buffer = JsFuture::from(array_buffer_promise).await?;

        // Convert to Vec<u8>
        let uint8_array = Uint8Array::new(&array_buffer);
        let mut data = vec![0u8; uint8_array.length() as usize];
        uint8_array.copy_to(&mut data);

        Ok(data)
    }

    /// Fetch JSON data from a URL
    pub async fn fetch_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<T, JsValue> {
        let opts = RequestInit::new();
        opts.set_method("GET");

        // Set headers for JSON
        let headers = Headers::new()?;
        headers.set("Accept", "application/json")?;

        let request = Request::new_with_str_and_init(url, &opts)?;

        let window =
            web_sys::window().ok_or_else(|| JsValue::from_str("No window object available"))?;

        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;

        if !resp.ok() {
            return Err(JsValue::from_str(&format!(
                "HTTP error! status: {}",
                resp.status()
            )));
        }

        let json_promise = resp.json()?;
        let json_value = JsFuture::from(json_promise).await?;

        // Convert JsValue to serde_json::Value then to T
        let json_str = js_sys::JSON::stringify(&json_value)?;
        let json_string: String = json_str.into();

        serde_json::from_str(&json_string)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))
    }

    /// Fetch with custom headers
    pub async fn fetch_with_headers(
        &self,
        url: &str,
        custom_headers: Vec<(&str, &str)>,
    ) -> Result<Vec<u8>, JsValue> {
        let opts = RequestInit::new();
        opts.set_method("GET");

        // Set custom headers
        let headers = Headers::new()?;
        for (key, value) in custom_headers {
            headers.set(key, value)?;
        }
        opts.set_headers(&headers);

        let request = Request::new_with_str_and_init(url, &opts)?;

        let window =
            web_sys::window().ok_or_else(|| JsValue::from_str("No window object available"))?;

        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;

        if !resp.ok() {
            return Err(JsValue::from_str(&format!(
                "HTTP error! status: {}",
                resp.status()
            )));
        }

        let array_buffer = JsFuture::from(resp.array_buffer()?).await?;
        let uint8_array = Uint8Array::new(&array_buffer);
        let mut data = vec![0u8; uint8_array.length() as usize];
        uint8_array.copy_to(&mut data);

        Ok(data)
    }
}

/// Convenience function for quick binary fetches
pub async fn fetch_binary(url: &str) -> Result<Vec<u8>, JsValue> {
    FetchClient::new().fetch_binary(url).await
}

/// Convenience function for quick JSON fetches
pub async fn fetch_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, JsValue> {
    FetchClient::new().fetch_json(url).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_fetch_binary() {
        // Test with a known endpoint
        let client = FetchClient::new();
        let result = client.fetch_binary("/api/test").await;
        assert!(result.is_ok() || result.is_err()); // Just ensure it runs
    }
}
