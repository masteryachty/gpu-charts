//! Cross-platform timing utilities that work in both WASM and native

#[cfg(target_arch = "wasm32")]
use web_sys::window;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

/// Cross-platform timer that works in both WASM and native environments
pub struct Timer {
    #[cfg(target_arch = "wasm32")]
    start: f64,
    #[cfg(not(target_arch = "wasm32"))]
    start: Instant,
}

impl Timer {
    /// Create a new timer starting now
    pub fn now() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            let start = window()
                .and_then(|w| w.performance())
                .map(|p| p.now())
                .unwrap_or(0.0);
            Self { start }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                start: Instant::now(),
            }
        }
    }

    /// Get elapsed time in seconds
    pub fn elapsed_secs(&self) -> f64 {
        #[cfg(target_arch = "wasm32")]
        {
            let now = window()
                .and_then(|w| w.performance())
                .map(|p| p.now())
                .unwrap_or(0.0);
            (now - self.start) / 1000.0 // Convert ms to seconds
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.start.elapsed().as_secs_f64()
        }
    }

    /// Get elapsed time in milliseconds
    pub fn elapsed_millis(&self) -> f64 {
        #[cfg(target_arch = "wasm32")]
        {
            let now = window()
                .and_then(|w| w.performance())
                .map(|p| p.now())
                .unwrap_or(0.0);
            now - self.start
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.start.elapsed().as_secs_f64() * 1000.0
        }
    }

    /// Get elapsed time as std::time::Duration (native only)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }
}

/// Get current timestamp in milliseconds since epoch
pub fn timestamp_millis() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as u64
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}
