# Shared Types Crate - Comprehensive Documentation

## Purpose and Responsibility

The `shared-types` crate is the foundational layer of the GPU Charts architecture, providing the common type system that enables communication and data flow between all other crates. As a zero-dependency foundation (within the workspace), it defines the lingua franca for the entire system, ensuring type safety and consistency across module boundaries.

### Core Responsibilities:
- **Type Definitions**: Provides all shared data structures, enums, and type aliases
- **Error System**: Defines the comprehensive error hierarchy for the entire application
- **Event System**: Implements custom event types for WebAssembly/browser integration
- **Data Contracts**: Establishes the data formats for server communication and GPU processing
- **Serialization Contracts**: Ensures consistent JSON serialization across WASM boundaries

## Architectural Position

```
shared-types (Foundation - Zero internal dependencies)
    ↑
    ├── config-system    (Configuration and quality presets)
    ├── data-manager     (Data operations and GPU buffers)
    ├── renderer         (GPU rendering engine)
    └── wasm-bridge      (JavaScript/React integration)
```

All other crates in the workspace depend on `shared-types`, but it depends on none of them, ensuring a clean dependency hierarchy.

## External Dependencies

```toml
# Core dependencies
serde = "1.0"          # Serialization framework for WASM boundary crossing
serde_json = "1.0"     # JSON serialization for JavaScript interop
uuid = "1.0"           # Unique identifiers with JavaScript compatibility
wgpu = "24.0.5"        # WebGPU types (specifically pinned version)
thiserror = "1.0"      # Error derivation macros
chrono = "0.4"         # Timestamp handling with WASM support

# WASM-specific dependencies (conditional compilation)
wasm-bindgen = "0.2"   # JavaScript binding generation
js-sys = "0.3"         # JavaScript standard library types
web-sys = "0.3"        # Browser API types
```

## Module Structure and Key Types

### 1. Core Library Module (`lib.rs`)

**Primary Types:**
- `DataHandle`: Unique handle for data sets with metadata
  - Contains UUID and metadata for tracking data lifecycle
  - Used by data-manager for buffer management
  
- `DataMetadata`: Comprehensive metadata for data sets
  - Fields: symbol, start_time, end_time, columns, row_count
  - Critical for data validation and cache management
  
- `ParsedData`: Container for parsed time-series data
  - Separates time data from value data using HashMap
  - Optimized for GPU buffer creation
  
- `WorldBounds`: Data space boundaries (f64 precision)
  - Used for coordinate transformations
  - Critical for zoom/pan calculations
  
- `ScreenBounds`: Rendering viewport dimensions (f32 precision)
  - Used for pixel-space calculations

### 2. Data Types Module (`data_types.rs`)

**Financial Data Structures:**
- `DataPoint`: Basic time-value pair (u32 timestamp, f32 value)
- `OhlcData`: Candlestick chart data (open, high, low, close, volume)
- `TradeData`: Individual trade information with side indicator
- `TradeSide`: Buy/Sell enum with lowercase serialization

**Data Management Types:**
- `ColumnType`: Strongly-typed column identifiers
  - Provides string conversion for API communication
  - Ensures type safety across data pipeline
  
- `DataRequest`: API request parameters
  - Fields: symbol, data_type, time range, columns
  
- `DataResponseHeader`: Server response metadata
  - Used for validating and parsing binary data streams

### 3. Error System Module (`errors.rs`)

**Comprehensive Error Hierarchy:**

The `GpuChartsError` enum provides categorized error types:

1. **Data Errors:**
   - `DataFetch`: Network/HTTP failures
   - `DataParse`: Parsing failures with offset tracking
   - `InvalidFormat`: Format mismatches
   - `DataNotFound`: Missing resources

2. **GPU/Rendering Errors:**
   - `GpuInit`: GPU initialization failures
   - `Surface`: Surface/swap chain errors
   - `BufferCreation`: GPU buffer allocation failures
   - `ShaderCompilation`: WGSL compilation errors
   - `RenderPipeline`: Pipeline creation failures

3. **Configuration Errors:**
   - `InvalidConfig`: Configuration validation failures
   - `MissingConfig`: Required fields missing

4. **State Management Errors:**
   - `StateValidation`: Multi-error validation results
   - `StateUpdate`: State mutation failures
   - `InstanceNotFound`: Missing chart instances

5. **Infrastructure Errors:**
   - `Network`: Network communication failures
   - `Timeout`: Operation timeouts with duration tracking
   - `JsInterop`: JavaScript boundary errors
   - `WasmMemory`: WASM memory issues

**Error Infrastructure:**
- `GpuChartsResult<T>`: Standard Result type alias
- `ErrorResponse`: Serializable error for JavaScript
- `ErrorContext`: Additional debugging information
- Conversion traits from external error types (wgpu, serde_json, JsValue)
- Helper macros: `gpu_error!` and `map_gpu_error!`

### 4. Event System Module (`events.rs`)

**Custom Event Types (Winit Replacement):**

Since winit doesn't work in WASM/browser contexts, this module provides lightweight event types:

- `PhysicalPosition`: Pixel-based coordinates (f64 precision)
- `MouseScrollDelta`: Scroll events (currently pixel-based only)
- `ElementState`: Pressed/Released states
- `MouseButton`: Mouse button identification (currently left only)
- `TouchPhase`: Touch event phases (currently moved only)
- `WindowEvent`: Unified event enum containing:
  - MouseWheel events with delta and phase
  - CursorMoved with position
  - MouseInput with state and button

Note: Many enum variants are commented out, suggesting a minimal implementation focused on essential functionality.

## Critical Implementation Patterns

### 1. Serialization Strategy
All public types implement `Serialize` and `Deserialize` to cross the WASM boundary:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataHandle {
    pub id: Uuid,
    pub metadata: DataMetadata,
}
```

### 2. Error Handling Pattern
The crate uses `thiserror` for ergonomic error definitions with automatic Display implementations:
```rust
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "details")]
pub enum GpuChartsError {
    #[error("Data fetch failed: {message}")]
    DataFetch { message: String },
}
```

### 3. Type Safety Through Enums
Strong typing prevents stringly-typed errors:
```rust
pub enum ColumnType {
    Time, BestBid, BestAsk, Price, Volume, Side,
    Open, High, Low, Close,
}
```

### 4. Platform-Specific Compilation
Conditional compilation for WASM-specific features:
```rust
#[cfg(target_arch = "wasm32")]
impl From<wasm_bindgen::JsValue> for GpuChartsError {
    fn from(err: wasm_bindgen::JsValue) -> Self {
        GpuChartsError::JsInterop {
            message: format!("{err:?}"),
        }
    }
}
```

## Testing Approach

The crate includes unit tests for critical functionality:

1. **Error Serialization Tests** (`errors.rs`):
   - Validates JSON serialization of error responses
   - Ensures error context is preserved
   - Tests error conversion traits

2. **Type Conversion Tests**:
   - Validates enum to string conversions
   - Tests serialization round-trips

Testing philosophy focuses on:
- Contract validation (serialization formats)
- Error handling correctness
- Type conversion accuracy

## Important Conventions

### 1. Timestamp Representation
- Uses `u32` for timestamps (Unix epoch seconds)
- Sufficient for financial data (covers until year 2106)
- Reduces memory usage compared to u64

### 2. Numeric Precision
- `f32` for GPU-bound data (values, prices)
- `f64` for world coordinates (higher precision needed)
- `u32` for counts and indices

### 3. Naming Conventions
- Snake_case for serialized field names
- PascalCase for type names
- Descriptive names avoiding abbreviations

### 4. Public API Design
- All fields are public for simplicity
- No hidden invariants in data structures
- Validation happens in consuming crates

## Performance Considerations

1. **Memory Layout**: Structures are designed to be cache-friendly with grouped related fields
2. **Copy Types**: Small enums implement Copy for efficient passing
3. **HashMap Usage**: Value data stored in HashMap for flexible column access
4. **String Allocations**: Minimized through use of `&'static str` where possible

## Integration Points

### With config-system:
- Provides error types for configuration validation
- Defines metadata structures for configuration storage

### With data-manager:
- Defines data request/response formats
- Provides parsed data structures for GPU buffer creation
- Error types for data operations

### With renderer:
- Provides world/screen bounds for transformations
- Surface error types from wgpu
- Event types for user interactions

### With wasm-bridge:
- All types are serializable for JavaScript boundary
- Error responses formatted for JavaScript consumption
- Event types map to browser events

## Future Considerations

Based on the current implementation, potential enhancements could include:

1. **Event System Expansion**: Uncommented event types suggest future support for right-click, middle-click, and complete touch gestures
2. **Performance Metrics**: Types for tracking frame times, GPU usage
3. **Streaming Data**: Types for real-time data updates
4. **Multi-Chart Support**: Extended metadata for multiple chart instances
5. **Custom Indicators**: Types for technical analysis indicators

## Migration and Breaking Changes

When modifying this crate:

1. **Adding Fields**: Use Option<T> for backward compatibility
2. **Changing Enums**: Add variants at the end to preserve serialization
3. **Removing Types**: Requires major version bump and migration in all dependent crates
4. **Serialization Changes**: Must maintain backward compatibility or provide migration path

## Key Insights

1. **Foundation Role**: This crate is truly foundational - changes ripple through the entire system
2. **Minimal Logic**: Contains almost no business logic, focusing on type definitions
3. **WASM-First Design**: Event system and error handling designed specifically for browser environment
4. **Performance Focus**: Type choices (u32 vs u64, f32 vs f64) show careful performance consideration
5. **Extensibility**: Commented code and optional fields suggest planned future features