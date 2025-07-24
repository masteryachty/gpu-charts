# Shared Types Crate - CLAUDE.md

This file provides guidance for working with the shared-types crate, which contains all common data structures and types used across the GPU Charts modular architecture.

## Overview

The shared-types crate is the foundational layer of the GPU Charts system, providing:
- Common data structures used by all other crates
- Type definitions for cross-crate communication
- Event system types for user interactions
- Store state management structures for React integration
- Error types and result definitions

## Architecture Position

```
shared-types (this crate)
    ↑
    ├── config-system
    ├── data-manager
    ├── renderer
    └── wasm-bridge
```

This crate has no dependencies on other workspace crates and serves as the common foundation.

## Key Modules

### Core Types (`src/lib.rs`)
- `ChartType`: Enum for chart visualization types (Line, Candlestick, Bar, Area)
- `GpuChartsConfig`: Main configuration structure
- `DataHandle`: Handle for managing data buffers
- `ParsedData`: Parsed time-series data structure
- `DataMetadata`: Metadata about loaded data
- `GpuBufferSet`: GPU buffer management structure

### Events Module (`src/events.rs`)
Provides winit-compatible event types for WebAssembly:
- `WindowEvent`: Mouse and keyboard events
- `MouseButton`, `ElementState`: Input state tracking
- `PhysicalPosition`: Coordinate system types
- `MouseScrollDelta`, `TouchPhase`: Scroll and touch events

### Store State Module (`src/store_state.rs`)
React store integration types:
- `StoreState`: Complete application state structure
- `ChartConfig`: Chart-specific configuration
- `MarketData`: Real-time market data structure
- `User`: User session information
- `ChangeDetectionConfig`: Smart change detection settings
- `StateChangeDetection`: Change detection results
- `StoreValidationResult`: State validation results

## Usage Patterns

### Adding New Shared Types

1. **Determine Module Placement**:
   - Core types → `lib.rs`
   - UI events → `events.rs`
   - React state → `store_state.rs`

2. **Follow Naming Conventions**:
   ```rust
   // Good: Clear, descriptive names
   pub struct ChartConfig { ... }
   pub enum ChartType { ... }
   
   // Avoid: Ambiguous or overly generic names
   pub struct Config { ... }
   pub enum Type { ... }
   ```

3. **Implement Required Traits**:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct NewType {
       // All shared types should be serializable
   }
   ```

### Store State Integration

The store state types enable seamless React-Rust communication:

```rust
// Validation example
impl StoreState {
    pub fn validate(&self) -> StoreValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // Add validation logic
        if self.chart_config.start_time >= self.chart_config.end_time {
            errors.push("Invalid time range".to_string());
        }
        
        StoreValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }
}
```

### Change Detection

The change detection system enables efficient updates:

```rust
impl StoreState {
    pub fn detect_changes_from(
        &self,
        previous: &StoreState,
        config: &ChangeDetectionConfig,
    ) -> StateChangeDetection {
        StateChangeDetection {
            has_changes: self != previous,
            symbol_changed: self.chart_config.symbol != previous.chart_config.symbol,
            // ... other change flags
        }
    }
}
```

## Best Practices

1. **Keep Types Simple**: Shared types should be POD (Plain Old Data) when possible
2. **Avoid Business Logic**: Keep logic in the appropriate crate, not in shared types
3. **Version Carefully**: Changes to shared types affect all crates
4. **Document Thoroughly**: All public types should have doc comments
5. **Use Semantic Versioning**: Breaking changes require major version bumps

## Common Patterns

### Result Types
```rust
pub type SharedResult<T> = Result<T, SharedError>;

#[derive(Debug, thiserror::Error)]
pub enum SharedError {
    #[error("Validation failed: {0}")]
    ValidationError(String),
    // Add other common errors
}
```

### Builder Pattern for Complex Types
```rust
impl ChartConfig {
    pub fn builder() -> ChartConfigBuilder {
        ChartConfigBuilder::default()
    }
}
```

## Testing

While shared-types primarily contains data structures, test:
- Serialization/deserialization roundtrips
- Validation logic
- Change detection accuracy
- Default implementations

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_store_state_validation() {
        let mut state = StoreState::default();
        state.chart_config.end_time = 0; // Invalid
        
        let result = state.validate();
        assert!(!result.is_valid);
    }
}
```

## Performance Considerations

- Keep structures small and cache-friendly
- Use `Arc` for large shared data
- Consider zero-copy serialization for IPC
- Minimize allocations in hot paths

## Future Enhancements

- Add protobuf support for binary serialization
- Implement schema versioning for migrations
- Add compile-time validation macros
- Consider const generics for buffer sizes