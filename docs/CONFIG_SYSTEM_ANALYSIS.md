# Config System Analysis and WASM Requirements

## Overview

The `config-system` crate provides a comprehensive configuration management system for GPU Charts with:
- Hierarchical configuration structure (rendering, data, performance, features, telemetry)
- Hot-reload capabilities for dynamic updates
- Auto-tuning to adjust quality based on performance
- Preset system for quick configuration changes
- Schema validation and parsing
- YAML/JSON configuration file support

## Current State

### What's Working
1. ✅ Core configuration structures (all serializable)
2. ✅ Default implementations for all configs
3. ✅ Preset management system
4. ✅ Parser for merging configurations
5. ✅ Schema module (already WASM-compatible)

### What's Broken/Incompatible with WASM
1. ❌ **Type Errors**: `u32 * float` operations in auto_tuning.rs
2. ❌ **File System Operations**: hot_reload.rs uses file watching (not available in WASM)
3. ❌ **System Info**: auto_tuning.rs uses sysinfo for hardware detection
4. ❌ **Missing Features**: Some modules aren't conditionally compiled for WASM

## Purpose in the System

The config-system serves as the central control mechanism:
```
JavaScript App → Config Update → WASM Bridge → Config System
                                                     ↓
                                              Update all subsystems:
                                              - Renderer settings
                                              - Data manager params
                                              - Performance tuning
```

### Key Responsibilities
1. **Configuration Management**: Store and validate all system settings
2. **Dynamic Updates**: Allow runtime configuration changes without restart
3. **Performance Tuning**: Automatically adjust quality based on FPS
4. **Preset System**: Quick switching between quality levels
5. **Validation**: Ensure configurations are valid and compatible

## What Needs to Be Done

### 1. Fix Type Errors (Immediate)
```rust
// Current (broken):
(config.performance.draw_call_batch_size * 1.5)

// Should be:
((config.performance.draw_call_batch_size as f32 * 1.5) as u32)
```

### 2. Make Hot Reload WASM-Compatible
- Remove file watching functionality for WASM
- Keep the hot reload manager but trigger updates via JavaScript
- Use broadcast channels that work in WASM

### 3. Fix Auto-Tuning Hardware Detection
```rust
// Replace sysinfo with web-sys
#[cfg(target_arch = "wasm32")]
use web_sys::{window, Navigator};

// Get hardware concurrency
let cpu_cores = navigator.hardware_concurrency();
```

### 4. Add WASM Features to Cargo.toml
```toml
[features]
default = ["wasm"]
wasm = []
native = ["notify", "sysinfo"]
```

### 5. Implement WASM-Specific Modules
- **Browser Storage**: Save/load configs from localStorage
- **Performance API**: Use browser Performance API for metrics
- **WebGPU Limits**: Query actual GPU capabilities

## Integration with WASM Bridge

The config-system will integrate with wasm-bridge like this:

```rust
// In wasm-bridge
let config_manager = Arc::new(HotReloadManager::new(
    default_config,
    |new_config| {
        // Apply config to all subsystems
        renderer.update_config(&new_config)?;
        data_manager.update_config(&new_config)?;
        Ok(())
    }
));

// JavaScript can update config
#[wasm_bindgen]
pub fn update_config(&self, config_json: &str) -> Result<()> {
    let config: GpuChartsConfig = serde_json::from_str(config_json)?;
    self.config_manager.update_config(config);
    Ok(())
}
```

## Priority Actions

1. **High Priority**: Fix the multiplication type errors in auto_tuning.rs
2. **High Priority**: Make hot_reload.rs conditional (native only) or WASM-compatible
3. **Medium Priority**: Replace sysinfo with web-sys for hardware detection
4. **Low Priority**: Add browser-specific features (localStorage, Performance API)

## Module-by-Module Status

### ✅ lib.rs
- Core types are all WASM-compatible
- Just needs unused import cleanup

### ❌ auto_tuning.rs
- Has type errors (u32 * float)
- Uses sysinfo (needs web-sys replacement)
- Needs hardware detection abstraction

### ❌ hot_reload.rs
- Uses file watching (notify crate)
- Needs to be made conditional or replaced with JS events

### ✅ parser.rs
- Pure data manipulation, should work in WASM
- Has unused variable warning to fix

### ✅ presets.rs
- Pure data manipulation, WASM-compatible

### ✅ schema.rs
- Already fixed for WASM (manual validation instead of jsonschema)

### ✅ system.rs
- Mostly WASM-compatible
- Just needs import cleanup

### ✅ validation.rs
- Pure validation logic, WASM-compatible
- Has unused variable warning

## Testing Strategy

1. **Unit Tests**: Test configuration parsing and validation
2. **Integration Tests**: Test config updates propagating to subsystems
3. **Browser Tests**: Test hot-reload via JavaScript
4. **Performance Tests**: Verify auto-tuning works with WebGPU

## Success Criteria

The config-system is ready when:
1. ✅ Compiles cleanly to WASM with no errors
2. ✅ Can parse and validate configurations
3. ✅ Supports runtime config updates from JavaScript
4. ✅ Auto-tuning works with WebGPU performance metrics
5. ✅ All presets apply correctly
6. ✅ Integrates smoothly with wasm-bridge

The config-system is relatively close to being WASM-ready - mainly needs fixing the type errors and making file-system operations conditional.