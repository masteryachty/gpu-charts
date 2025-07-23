# Extra Crates Cleanup Plan

## Analysis of Extra Crates

### Crates to Remove (Redundant)

1. **wasm-bridge-minimal**
   - Purpose: Simplified demo of Phase 3 config features
   - Redundant because: We have a clean wasm-bridge implementation with config support
   - Action: Remove from workspace and delete

2. **gpu-charts-unified**
   - Purpose: Binary search culling implementation
   - Redundant because: Already implemented in renderer/src/culling.rs
   - Action: Remove from workspace and delete

3. **wasm-storage**
   - Purpose: Simple storage wrapper (only 5 lines)
   - Redundant because: Too minimal to be useful
   - Action: Remove from workspace and delete

4. **system-integration**
   - Status: Already removed from workspace in Option 1
   - Action: Delete the directory

### Crates to Keep (Potentially Useful)

1. **wasm-fetch**
   - Purpose: Clean WASM-compatible HTTP client using browser's fetch API
   - Useful for: data-manager's HTTP fetching in WASM environment
   - Action: Keep and integrate into data-manager for WASM builds

2. **wasm-websocket**
   - Purpose: WASM-compatible WebSocket wrapper
   - Useful for: data-manager's WebSocket support in WASM environment
   - Action: Keep and integrate into data-manager for WASM builds

## Implementation Steps

### Step 1: Remove Redundant Crates
```bash
# Remove from workspace Cargo.toml
# Remove directories:
rm -rf crates/wasm-bridge-minimal
rm -rf crates/gpu-charts-unified
rm -rf crates/wasm-storage
rm -rf crates/system-integration
```

### Step 2: Integrate Useful Crates
- Move wasm-fetch functionality into data-manager as a module
- Move wasm-websocket functionality into data-manager as a module
- Update data-manager to use these modules when building for WASM

### Step 3: Update Workspace
```toml
[workspace]
members = [
    # Core WASM crates (following NEW_ARCHITECTURE.md)
    "crates/shared-types",
    "crates/data-manager",
    "crates/renderer", 
    "crates/wasm-bridge",
    
    # Additional useful crates
    "crates/config-system",
    
    # Existing crates
    "charting",
    "server",
    "coinbase-logger",
    "file_server",
    
    # Benchmarking
    "benchmarks"
]
```

## Benefits

1. **Cleaner Architecture**: Removes 4 redundant crates
2. **Better Organization**: HTTP and WebSocket functionality integrated where needed
3. **Follows NEW_ARCHITECTURE.md**: Maintains the 3+1 component design
4. **Reduced Complexity**: Fewer crates to maintain and understand