//! Public API module with deprecation warnings and versioning

use crate::unified_api::{ApiVersion, UnifiedApi};
use crate::Result;
use std::sync::Arc;

/// Main API entry point with versioning support
pub struct GpuChartsApi {
    /// Internal unified API
    api: Arc<UnifiedApi>,

    /// Deprecation tracker
    deprecations: DeprecationTracker,
}

impl GpuChartsApi {
    /// Create a new API instance
    pub fn new(api: UnifiedApi) -> Self {
        Self {
            api: Arc::new(api),
            deprecations: DeprecationTracker::new(),
        }
    }

    /// Get API version
    pub fn version(&self) -> ApiVersion {
        self.api.version().clone()
    }

    /// Check if a feature is deprecated
    pub fn is_deprecated(&self, feature: &str) -> bool {
        self.deprecations.is_deprecated(feature)
    }

    /// Get deprecation warnings
    pub fn get_deprecation_warnings(&self) -> Vec<DeprecationWarning> {
        self.deprecations.get_warnings()
    }
}

/// Deprecation tracking
struct DeprecationTracker {
    warnings: Vec<DeprecationWarning>,
}

impl DeprecationTracker {
    fn new() -> Self {
        let mut tracker = Self {
            warnings: Vec::new(),
        };

        // Add known deprecations
        tracker.add_deprecation(DeprecationWarning {
            feature: "createChartSync".to_string(),
            since_version: "0.9.0".to_string(),
            removal_version: Some("2.0.0".to_string()),
            alternative: "Use createChart (async) instead".to_string(),
        });

        tracker
    }

    fn add_deprecation(&mut self, warning: DeprecationWarning) {
        self.warnings.push(warning);
    }

    fn is_deprecated(&self, feature: &str) -> bool {
        self.warnings.iter().any(|w| w.feature == feature)
    }

    fn get_warnings(&self) -> Vec<DeprecationWarning> {
        self.warnings.clone()
    }
}

/// Deprecation warning information
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeprecationWarning {
    pub feature: String,
    pub since_version: String,
    pub removal_version: Option<String>,
    pub alternative: String,
}

/// API compatibility layer for older versions
pub mod v0 {
    use super::*;

    /// Version 0.x compatibility API
    pub struct GpuChartsApiV0 {
        api: Arc<UnifiedApi>,
    }

    impl GpuChartsApiV0 {
        pub fn new(api: UnifiedApi) -> Self {
            Self { api: Arc::new(api) }
        }

        /// Legacy create chart method (synchronous wrapper)
        #[deprecated(since = "0.9.0", note = "Use async createChart instead")]
        pub fn create_chart_sync(&self, config: serde_json::Value) -> Result<String> {
            log::warn!("Using deprecated synchronous API");

            // This would block on async operation - not recommended
            Err(crate::IntegrationError::Bridge(
                "Synchronous API no longer supported".to_string(),
            ))
        }
    }
}

/// OpenAPI specification generator
pub struct OpenApiGenerator;

impl OpenApiGenerator {
    /// Generate OpenAPI 3.0 specification
    pub fn generate() -> serde_json::Value {
        serde_json::json!({
            "openapi": "3.0.0",
            "info": {
                "title": "GPU Charts API",
                "version": "1.0.0",
                "description": "High-performance GPU-accelerated charting library"
            },
            "servers": [
                {
                    "url": "https://api.gpucharts.io/v1",
                    "description": "Production server"
                }
            ],
            "paths": {
                "/charts": {
                    "post": {
                        "summary": "Create a new chart",
                        "operationId": "createChart",
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ChartConfiguration"
                                    }
                                }
                            }
                        },
                        "responses": {
                            "201": {
                                "description": "Chart created successfully",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "$ref": "#/components/schemas/ChartHandle"
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "get": {
                        "summary": "List all charts",
                        "operationId": "listCharts",
                        "responses": {
                            "200": {
                                "description": "List of charts",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "array",
                                            "items": {
                                                "$ref": "#/components/schemas/ChartInfo"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "/charts/{chartId}": {
                    "get": {
                        "summary": "Get chart information",
                        "operationId": "getChart",
                        "parameters": [
                            {
                                "name": "chartId",
                                "in": "path",
                                "required": true,
                                "schema": {
                                    "type": "string",
                                    "format": "uuid"
                                }
                            }
                        ],
                        "responses": {
                            "200": {
                                "description": "Chart information",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "$ref": "#/components/schemas/ChartInfo"
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "delete": {
                        "summary": "Delete a chart",
                        "operationId": "deleteChart",
                        "parameters": [
                            {
                                "name": "chartId",
                                "in": "path",
                                "required": true,
                                "schema": {
                                    "type": "string",
                                    "format": "uuid"
                                }
                            }
                        ],
                        "responses": {
                            "204": {
                                "description": "Chart deleted successfully"
                            }
                        }
                    }
                },
                "/charts/{chartId}/data": {
                    "post": {
                        "summary": "Load data into a chart",
                        "operationId": "loadChartData",
                        "parameters": [
                            {
                                "name": "chartId",
                                "in": "path",
                                "required": true,
                                "schema": {
                                    "type": "string",
                                    "format": "uuid"
                                }
                            }
                        ],
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/DataLoadRequest"
                                    }
                                }
                            }
                        },
                        "responses": {
                            "201": {
                                "description": "Data loaded successfully",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object",
                                            "properties": {
                                                "dataId": {
                                                    "type": "string",
                                                    "format": "uuid"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "/charts/{chartId}/viewport": {
                    "put": {
                        "summary": "Update chart viewport",
                        "operationId": "updateViewport",
                        "parameters": [
                            {
                                "name": "chartId",
                                "in": "path",
                                "required": true,
                                "schema": {
                                    "type": "string",
                                    "format": "uuid"
                                }
                            }
                        ],
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/Viewport"
                                    }
                                }
                            }
                        },
                        "responses": {
                            "200": {
                                "description": "Viewport updated successfully"
                            }
                        }
                    }
                }
            },
            "components": {
                "schemas": {
                    "ChartConfiguration": {
                        "type": "object",
                        "required": ["chartType"],
                        "properties": {
                            "chartType": {
                                "type": "string",
                                "enum": ["Line", "Scatter", "Heatmap", "ThreeD"]
                            },
                            "visualConfig": {
                                "$ref": "#/components/schemas/VisualConfig"
                            },
                            "overlays": {
                                "type": "array",
                                "items": {
                                    "$ref": "#/components/schemas/OverlayConfig"
                                }
                            }
                        }
                    },
                    "VisualConfig": {
                        "type": "object",
                        "properties": {
                            "backgroundColor": {
                                "type": "array",
                                "items": {
                                    "type": "number"
                                },
                                "minItems": 4,
                                "maxItems": 4
                            },
                            "gridColor": {
                                "type": "array",
                                "items": {
                                    "type": "number"
                                },
                                "minItems": 4,
                                "maxItems": 4
                            },
                            "textColor": {
                                "type": "array",
                                "items": {
                                    "type": "number"
                                },
                                "minItems": 4,
                                "maxItems": 4
                            },
                            "marginPercent": {
                                "type": "number",
                                "minimum": 0,
                                "maximum": 1
                            },
                            "showGrid": {
                                "type": "boolean"
                            },
                            "showAxes": {
                                "type": "boolean"
                            }
                        }
                    },
                    "OverlayConfig": {
                        "type": "object",
                        "required": ["overlayType", "renderLocation"],
                        "properties": {
                            "overlayType": {
                                "type": "string"
                            },
                            "renderLocation": {
                                "type": "string",
                                "enum": ["AbovePlot", "BelowPlot"]
                            }
                        }
                    },
                    "ChartHandle": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "format": "uuid"
                            }
                        }
                    },
                    "ChartInfo": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "format": "uuid"
                            },
                            "chartType": {
                                "type": "string"
                            },
                            "dataCount": {
                                "type": "integer"
                            },
                            "viewport": {
                                "$ref": "#/components/schemas/Viewport"
                            }
                        }
                    },
                    "DataLoadRequest": {
                        "type": "object",
                        "required": ["source", "metadata"],
                        "properties": {
                            "source": {
                                "$ref": "#/components/schemas/DataSource"
                            },
                            "metadata": {
                                "$ref": "#/components/schemas/BufferMetadata"
                            }
                        }
                    },
                    "DataSource": {
                        "type": "object",
                        "required": ["type"],
                        "properties": {
                            "type": {
                                "type": "string",
                                "enum": ["Http", "WebSocket", "File"]
                            },
                            "url": {
                                "type": "string",
                                "format": "uri"
                            },
                            "path": {
                                "type": "string"
                            }
                        }
                    },
                    "BufferMetadata": {
                        "type": "object",
                        "required": ["rowCount", "columnCount", "timeRange", "valueRange"],
                        "properties": {
                            "rowCount": {
                                "type": "integer",
                                "minimum": 0
                            },
                            "columnCount": {
                                "type": "integer",
                                "minimum": 1
                            },
                            "timeRange": {
                                "type": "array",
                                "items": {
                                    "type": "number"
                                },
                                "minItems": 2,
                                "maxItems": 2
                            },
                            "valueRange": {
                                "type": "array",
                                "items": {
                                    "type": "number"
                                },
                                "minItems": 2,
                                "maxItems": 2
                            }
                        }
                    },
                    "Viewport": {
                        "type": "object",
                        "required": ["xMin", "xMax", "yMin", "yMax"],
                        "properties": {
                            "xMin": {
                                "type": "number"
                            },
                            "xMax": {
                                "type": "number"
                            },
                            "yMin": {
                                "type": "number"
                            },
                            "yMax": {
                                "type": "number"
                            }
                        }
                    }
                }
            }
        })
    }
}
