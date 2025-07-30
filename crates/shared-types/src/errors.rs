//! Common error types used across all GPU Charts crates
//! Provides consistent error handling and reporting

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Base error type for all GPU Charts operations
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "details")]
pub enum GpuChartsError {
    // Data-related errors
    #[error("Data fetch failed: {message}")]
    DataFetch { message: String },

    #[error("Data parse error: {message}")]
    DataParse {
        message: String,
        offset: Option<usize>,
    },

    #[error("Invalid data format: {expected} but got {actual}")]
    InvalidFormat { expected: String, actual: String },

    #[error("Data not found: {resource}")]
    DataNotFound { resource: String },

    // GPU/Rendering errors
    #[error("GPU initialization failed: {message}")]
    GpuInit { message: String },

    #[error("Surface error: {message}")]
    Surface { message: String },

    #[error("Buffer creation failed: {message}")]
    BufferCreation { message: String, size: Option<u64> },

    #[error("Shader compilation failed: {message}")]
    ShaderCompilation { message: String, shader: String },

    #[error("Render pipeline error: {message}")]
    RenderPipeline { message: String },

    // Configuration errors
    #[error("Invalid configuration: {message}")]
    InvalidConfig {
        message: String,
        field: Option<String>,
    },

    #[error("Missing required configuration: {field}")]
    MissingConfig { field: String },

    // State management errors
    #[error("State validation failed: {errors:?}")]
    StateValidation {
        errors: Vec<String>,
        warnings: Vec<String>,
    },

    #[error("State update failed: {message}")]
    StateUpdate { message: String },

    #[error("Instance not found: {id}")]
    InstanceNotFound { id: String },

    // Network errors
    #[error("Network request failed: {message}")]
    Network { message: String },

    #[error("Request timeout: {message}")]
    Timeout { message: String, duration_ms: u64 },

    // WASM-specific errors
    #[error("JavaScript interop error: {message}")]
    JsInterop { message: String },

    #[error("WASM memory error: {message}")]
    WasmMemory { message: String },

    // Generic errors
    #[error("Operation cancelled")]
    Cancelled,

    #[error("Not implemented: {feature}")]
    NotImplemented { feature: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

/// Result type alias for GPU Charts operations
pub type GpuChartsResult<T> = Result<T, GpuChartsError>;

/// Error response structure for JavaScript interop
#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: GpuChartsError,
    pub timestamp: u64,
    pub context: Option<ErrorContext>,
}

/// Additional context for error reporting
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ErrorContext {
    pub component: String,
    pub operation: String,
    pub metadata: serde_json::Value,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(error: GpuChartsError) -> Self {
        Self {
            success: false,
            error,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            context: None,
        }
    }

    /// Add context to the error response
    pub fn with_context(mut self, component: &str, operation: &str) -> Self {
        self.context = Some(ErrorContext {
            component: component.to_string(),
            operation: operation.to_string(),
            metadata: serde_json::Value::Null,
        });
        self
    }

    /// Add metadata to the error context
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        if let Some(ref mut ctx) = self.context {
            ctx.metadata = metadata;
        }
        self
    }

    /// Convert to JSON string for JavaScript
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"success":false,"error":{"type":"Internal","details":{"message":"Failed to serialize error"}}}"#.to_string()
        })
    }
}

/// Trait for converting various error types to GpuChartsError
pub trait ToGpuChartsError {
    fn to_gpu_charts_error(self) -> GpuChartsError;
}

// Implement conversions for common error types
impl From<wgpu::SurfaceError> for GpuChartsError {
    fn from(err: wgpu::SurfaceError) -> Self {
        GpuChartsError::Surface {
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for GpuChartsError {
    fn from(err: serde_json::Error) -> Self {
        GpuChartsError::DataParse {
            message: err.to_string(),
            offset: Some(err.line()),
        }
    }
}

impl From<wasm_bindgen::JsValue> for GpuChartsError {
    fn from(err: wasm_bindgen::JsValue) -> Self {
        GpuChartsError::JsInterop {
            message: format!("{err:?}"),
        }
    }
}

/// Helper macro for creating errors with context
#[macro_export]
macro_rules! gpu_error {
    ($variant:ident { $($field:ident: $value:expr),* }) => {
        $crate::errors::GpuChartsError::$variant {
            $($field: $value.into()),*
        }
    };
}

/// Helper macro for converting Results to GpuChartsResult
#[macro_export]
macro_rules! map_gpu_error {
    ($result:expr, $error_variant:ident, $message:expr) => {
        $result.map_err(|e| $crate::errors::GpuChartsError::$error_variant {
            message: format!("{}: {}", $message, e),
        })
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_serialization() {
        let error = GpuChartsError::DataFetch {
            message: "Failed to connect (URL: https://api.example.com)".to_string(),
        };

        let response = ErrorResponse::new(error).with_context("DataManager", "fetch_data");

        let json = response.to_json();
        assert!(json.contains("DataFetch"));
        assert!(json.contains("Failed to connect"));
    }

    #[test]
    fn test_error_conversion() {
        let surface_err = wgpu::SurfaceError::Outdated;
        let gpu_err: GpuChartsError = surface_err.into();

        match gpu_err {
            GpuChartsError::Surface { message } => {
                assert!(message.contains("Outdated"));
            }
            _ => panic!("Wrong error variant"),
        }
    }
}
