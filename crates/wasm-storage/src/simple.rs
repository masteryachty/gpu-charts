use wasm_bindgen::prelude::*;

/// Simple browser storage using only LocalStorage and SessionStorage
pub struct SimpleStorage {
    use_session: bool,
}

impl SimpleStorage {
    /// Create storage using LocalStorage
    pub fn local() -> Self {
        Self { use_session: false }
    }

    /// Create storage using SessionStorage
    pub fn session() -> Self {
        Self { use_session: true }
    }

    /// Store a string value
    pub fn set(&self, key: &str, value: &str) -> Result<(), JsValue> {
        let storage = self.get_storage()?;
        storage.set_item(key, value)
    }

    /// Get a string value
    pub fn get(&self, key: &str) -> Result<Option<String>, JsValue> {
        let storage = self.get_storage()?;
        storage.get_item(key)
    }

    /// Store JSON data
    pub fn set_json<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<(), JsValue> {
        let json = serde_json::to_string(value)
            .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))?;
        self.set(key, &json)
    }

    /// Get JSON data
    pub fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, JsValue> {
        match self.get(key)? {
            Some(json) => {
                let value = serde_json::from_str(&json)
                    .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Remove a value
    pub fn remove(&self, key: &str) -> Result<(), JsValue> {
        let storage = self.get_storage()?;
        storage.remove_item(key)
    }

    /// Clear all storage
    pub fn clear(&self) -> Result<(), JsValue> {
        let storage = self.get_storage()?;
        storage.clear()
    }

    /// Check if a key exists
    pub fn has(&self, key: &str) -> Result<bool, JsValue> {
        Ok(self.get(key)?.is_some())
    }

    /// Get all keys
    pub fn keys(&self) -> Result<Vec<String>, JsValue> {
        let storage = self.get_storage()?;
        let mut keys = Vec::new();

        for i in 0..storage.length()? {
            if let Some(key) = storage.key(i)? {
                keys.push(key);
            }
        }

        Ok(keys)
    }

    fn get_storage(&self) -> Result<web_sys::Storage, JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window object"))?;

        if self.use_session {
            window
                .session_storage()
                .map_err(|_| JsValue::from_str("SessionStorage not available"))?
                .ok_or_else(|| JsValue::from_str("SessionStorage not supported"))
        } else {
            window
                .local_storage()
                .map_err(|_| JsValue::from_str("LocalStorage not available"))?
                .ok_or_else(|| JsValue::from_str("LocalStorage not supported"))
        }
    }
}
