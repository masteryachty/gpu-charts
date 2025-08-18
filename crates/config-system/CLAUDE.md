# Config System Crate - CLAUDE.md

This file provides comprehensive guidance for working with the config-system crate, which manages chart rendering presets and visualization configurations for the GPU Charts system.

## Purpose and Responsibility

The config-system crate is responsible for:
- **Chart Preset Management**: Defining and managing pre-configured chart visualization setups
- **Render Type Definitions**: Specifying different visualization types (Line, Bar, Candlestick, Triangle, Area)
- **Style Configuration**: Managing visual appearance settings like colors, sizes, and opacity
- **Compute Operations**: Defining calculated fields like averages, sums, and weighted calculations
- **Data Column Mapping**: Linking data sources to visualization components

This crate acts as the single source of truth for all chart configuration patterns, ensuring consistency across the application.

## Architecture Position

```
shared-types (foundation layer)
    ↑
config-system (this crate - configuration layer)
    ↑
Used by: data-manager, renderer, wasm-bridge
```

### Dependencies
- **shared-types**: Provides base types (only internal dependency)
- **serde/serde_json**: For serialization of configuration structures
- **wasm-bindgen**: WebAssembly bindings (conditional for wasm32 targets)

## Core Configuration Architecture

### 1. Chart Presets (`ChartPreset`)

A preset represents a complete chart configuration with multiple visualization layers:

```rust
pub struct ChartPreset {
    pub name: String,              // Unique identifier (e.g., "Market Data")
    pub description: String,        // Human-readable description
    pub chart_types: Vec<RenderPreset>, // Multiple render layers
}
```

### 2. Render Presets (`RenderPreset`)

Each render preset defines a single visualization layer within a chart:

```rust
pub struct RenderPreset {
    pub render_type: RenderType,           // Visualization type
    pub data_columns: Vec<(String, String)>, // Primary data source
    pub additional_data_columns: Option<Vec<(String, String)>>, // Supplementary data
    pub visible: bool,                      // Default visibility
    pub label: String,                      // Display name
    pub style: RenderStyle,                 // Visual appearance
    pub compute_op: Option<ComputeOp>,      // Calculation method
}
```

### 3. Render Types (`RenderType`)

Five fundamental visualization types:

```rust
pub enum RenderType {
    Line,        // Continuous line plots
    Bar,         // Bar/histogram charts
    Candlestick, // OHLC financial charts
    Triangle,    // Point markers (typically for trades)
    Area,        // Filled area charts
}
```

### 4. Style Configuration (`RenderStyle`)

Visual appearance settings:

```rust
pub struct RenderStyle {
    pub color: Option<[f32; 4]>,           // Single RGBA color
    pub color_options: Option<Vec<[f32; 4]>>, // Multiple colors for conditional rendering
    pub size: f32,                          // Width/size parameter
}
```

### 5. Compute Operations (`ComputeOp`)

Mathematical operations for calculated fields:

```rust
pub enum ComputeOp {
    Average,                             // Mean of inputs
    Sum,                                // Total of inputs
    Difference,                         // Subtraction (a - b)
    Product,                           // Multiplication
    Ratio,                             // Division (a / b)
    Min,                               // Minimum value
    Max,                               // Maximum value
    WeightedAverage { weights: Vec<f32> }, // Weighted mean
}
```

## Preset System Structure

### Available Presets

The system provides two main preset categories:

#### 1. Market Data Preset (`market_data_presets.rs`)

Comprehensive financial market visualization with four components:

```rust
"Market Data" preset:
├── Bid Line (green, initially hidden)
│   └── Data: MD.best_bid
├── Ask Line (red, initially hidden)
│   └── Data: MD.best_ask
├── Trade Triangles (conditional coloring, initially hidden)
│   ├── Data: TRADES.price
│   └── Additional: TRADES.side (for buy/sell coloring)
└── Mid Price Line (blue, visible by default)
    ├── Data: COMPUTED.Mid
    ├── Additional: MD.best_ask, MD.best_bid
    └── Compute: Average (bid + ask) / 2
```

**Key Features:**
- Bid/Ask spread visualization
- Trade execution markers with buy/sell distinction
- Calculated mid-price with automatic averaging
- Configurable visibility for each component

#### 2. Candlestick Preset (`candle_presets.rs`)

Traditional OHLC candlestick chart:

```rust
"Candlestick" preset:
└── OHLC Candlesticks (green base color)
    └── Data: TRADES.price (aggregated into OHLC)
```

**Key Features:**
- Automatic OHLC aggregation from trade data
- Configurable body width relative to time interval
- Green/red coloring for bullish/bearish candles

## Data Column Mapping System

### Data Source Format

Data columns follow the pattern: `(data_type, column_name)`

**Standard Data Types:**
- `"MD"`: Market data (quotes)
- `"TRADES"`: Trade executions
- `"COMPUTED"`: Calculated fields

**Common Columns:**
- Market Data: `best_bid`, `best_ask`, `volume`
- Trades: `price`, `side`, `volume`
- Computed: `Mid`, `VWAP`, custom calculations

### Column Usage Patterns

1. **Primary Data Columns** (`data_columns`):
   - Used for Y-axis bounds calculation
   - Directly mapped to visualization coordinates

2. **Additional Data Columns** (`additional_data_columns`):
   - Supplementary data not affecting Y bounds
   - Used for styling decisions (e.g., trade side for coloring)
   - Input for computed fields

## PresetManager API

The `PresetManager` provides centralized access to all presets:

```rust
impl PresetManager {
    // Initialize with all default presets
    pub fn new() -> Self
    
    // List all available preset names
    pub fn list_presets_by_name(&self) -> Vec<&str>
    
    // Get all preset configurations
    pub fn get_all_presets(&self) -> &[ChartPreset]
    
    // Find specific preset by name
    pub fn find_preset(&self, name: &str) -> Option<&ChartPreset>
    
    // Get metrics/labels for a preset
    pub fn get_metrics_for_preset(&self, name: &str) -> Vec<&str>
}
```

## Usage Examples

### Basic Preset Usage

```rust
use config_system::PresetManager;

// Initialize preset manager
let preset_manager = PresetManager::new();

// List available presets
let preset_names = preset_manager.list_presets_by_name();
// Returns: ["Market Data", "Candlestick"]

// Get specific preset
if let Some(preset) = preset_manager.find_preset("Market Data") {
    for chart_type in &preset.chart_types {
        println!("Component: {} ({})", chart_type.label, chart_type.render_type);
    }
}
```

### Creating Custom Presets

```rust
use config_system::{ChartPreset, RenderPreset, RenderStyle, RenderType};

let custom_preset = ChartPreset {
    name: "Volume Profile".to_string(),
    description: "Volume-based visualization".to_string(),
    chart_types: vec![
        RenderPreset {
            render_type: RenderType::Bar,
            data_columns: vec![("TRADES".to_string(), "volume".to_string())],
            additional_data_columns: None,
            visible: true,
            label: "Volume".to_string(),
            style: RenderStyle {
                color: Some([0.5, 0.5, 0.8, 0.7]), // Semi-transparent blue
                color_options: None,
                size: 0.9, // Bar width
            },
            compute_op: None,
        }
    ],
};
```

### Working with Computed Fields

```rust
// Example: Creating a VWAP (Volume-Weighted Average Price) component
RenderPreset {
    render_type: RenderType::Line,
    data_columns: vec![("COMPUTED".to_string(), "VWAP".to_string())],
    additional_data_columns: Some(vec![
        ("TRADES".to_string(), "price".to_string()),
        ("TRADES".to_string(), "volume".to_string()),
    ]),
    visible: true,
    label: "VWAP".to_string(),
    style: RenderStyle {
        color: Some([1.0, 0.5, 0.0, 1.0]), // Orange
        color_options: None,
        size: 2.0,
    },
    compute_op: Some(ComputeOp::WeightedAverage { 
        weights: vec![] // Weights derived from volume data
    }),
}
```

## Integration with Other Crates

### Data Manager Integration

The data-manager crate uses presets to:
- Determine which data columns to fetch
- Prepare GPU buffers for each render component
- Calculate bounds based on visible components

### Renderer Integration

The renderer crate uses presets to:
- Select appropriate rendering pipelines
- Apply style configurations
- Manage render passes for each component

### WASM Bridge Integration

The wasm-bridge uses presets to:
- Expose configuration to JavaScript/React
- Handle preset switching from UI
- Synchronize visibility toggles

## Important Patterns and Best Practices

### 1. Preset Composition

Presets are composable - a single preset can contain multiple render layers:
```rust
// Good: Composite preset with related visualizations
ChartPreset {
    name: "Full Market View",
    chart_types: vec![bid, ask, trades, volume, mid_price],
}

// Avoid: Mixing unrelated visualizations
```

### 2. Color Management

Use consistent color schemes:
```rust
// Standard color palette
const GREEN: [f32; 4] = [0.0, 0.8, 0.0, 1.0];  // Bullish/Buy
const RED: [f32; 4] = [0.8, 0.0, 0.0, 1.0];    // Bearish/Sell
const BLUE: [f32; 4] = [0.0, 0.5, 1.0, 1.0];   // Neutral/Info
```

### 3. Visibility Defaults

Set sensible default visibility:
```rust
// Show primary data by default
visible: true,  // For main price/value lines

// Hide supplementary data
visible: false, // For bid/ask/trades that may clutter
```

### 4. Data Column Validation

Always validate data column availability:
```rust
// Check if required columns exist before rendering
if data_manager.has_column("MD", "best_bid") {
    // Safe to use bid line preset
}
```

### 5. Compute Operation Efficiency

Place computed fields strategically:
```rust
// Good: Compute once, display result
compute_op: Some(ComputeOp::Average)

// Avoid: Redundant calculations in multiple places
```

## Performance Considerations

1. **Preset Switching**: Changing presets may require GPU buffer reallocation
2. **Visibility Toggles**: Use visibility flags for fast show/hide without data reload
3. **Computed Fields**: Cache results of expensive computations
4. **Color Arrays**: Pre-allocate color options to avoid runtime allocation

## Testing Guidelines

### Unit Tests

Each preset module includes tests:
```rust
#[test]
fn test_preset_structure() {
    let preset = create_market_data_presets();
    assert_eq!(preset.chart_types.len(), 4);
    // Verify each component
}

#[test]
fn test_compute_operations() {
    // Test that computed fields have correct operations
}
```

### Integration Tests

Test preset usage across crates:
```rust
#[test]
fn test_preset_to_gpu_buffer_conversion() {
    let preset = PresetManager::new().find_preset("Market Data").unwrap();
    // Verify data-manager can process preset
    // Verify renderer can render preset
}
```

## Future Enhancements

### Planned Features
1. **Dynamic Preset Loading**: Load presets from JSON/YAML files
2. **User-Defined Presets**: Allow users to save custom configurations
3. **Preset Inheritance**: Base presets with overrides
4. **Conditional Presets**: Different presets based on data characteristics
5. **Performance Presets**: Automatic quality adjustment based on data volume

### Potential Improvements
- Hot-reload preset changes without restart
- Preset validation and error reporting
- A/B testing framework for preset effectiveness
- Machine learning-based preset recommendations
- Preset templates for common chart patterns

## Troubleshooting

### Common Issues

1. **Preset Not Found**
   ```rust
   // Always check if preset exists
   if let Some(preset) = manager.find_preset(name) {
       // Use preset
   } else {
       // Handle missing preset
   }
   ```

2. **Missing Data Columns**
   - Verify data source provides required columns
   - Check column naming matches exactly

3. **Compute Operation Failures**
   - Ensure additional_data_columns provides required inputs
   - Validate compute operation compatibility with data types

4. **Color Rendering Issues**
   - RGBA values must be in [0.0, 1.0] range
   - Alpha channel affects transparency

## Summary

The config-system crate provides a flexible, extensible framework for managing chart visualization configurations. By centralizing preset definitions and providing a clean API, it enables consistent, maintainable chart rendering across the entire GPU Charts application. The separation of data sources, render types, and visual styles allows for powerful composition while maintaining simplicity in usage.