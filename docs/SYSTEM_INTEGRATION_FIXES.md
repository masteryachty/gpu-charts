# System Integration Fixes Summary

## Issues Found

### 1. DataManager API Mismatch
- **Issue**: bridge.rs calls `load_data()` but DataManager has `fetch_data()`
- **Fix**: Either update bridge to use `fetch_data()` or create adapter method

### 2. Viewport Field Mismatch
- **Issue**: Code expects `x_min`, `x_max` but Viewport has `x`, `width`
- **Fix**: Calculate min/max from x/width: `x_min = x`, `x_max = x + width`

### 3. Missing Subsystem Hash Implementation
- **Issue**: `Subsystem` enum needs Hash trait for HashMap usage
- **Fix**: Add `#[derive(Hash)]` to Subsystem enum

### 4. BufferHandle Missing Methods
- **Issue**: `get_buffer_set()` method doesn't exist on BufferHandle
- **Fix**: Use `access()` method instead which returns Option<Arc<BufferData>>

### 5. Viewport Missing Default
- **Issue**: Code calls `Viewport::default()` but no Default impl
- **Fix**: Either add Default impl or construct manually

## Quick Fixes Needed

```rust
// 1. Fix DataManager usage
// Instead of: data_manager.load_data(source, metadata)
// Use: data_manager.fetch_data(&request_json)

// 2. Fix Viewport usage
// Instead of: viewport.x_min
// Use: viewport.x
// Instead of: viewport.x_max  
// Use: viewport.x + viewport.width

// 3. Add to Subsystem enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Subsystem { ... }

// 4. Fix BufferHandle usage
// Instead of: handle.get_buffer_set()
// Use: handle.access()

// 5. Add Viewport::default() or construct manually
impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
            zoom_level: 1.0,
            time_range: TimeRange::new(0, 0),
        }
    }
}
```

## Root Cause

The system-integration crate was written against an older or imagined API that doesn't match the actual implementations. It needs to be updated to use the real APIs from data-manager and renderer crates.