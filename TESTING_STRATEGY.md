# WASM Rust Testing Strategy

## Overview

This document outlines the testing approach for Rust WebAssembly projects using wasm-pack, based on research and implementation experience.

## Key Testing Insights

### 1. **Dual Target Architecture**
```toml
[lib]
crate-type = ["cdylib", "rlib"]
```
- `cdylib`: For WASM compilation and browser deployment
- `rlib`: For native Rust library compilation and testing

### 2. **Testing Levels**

#### **Native Testing** (`cargo test --target x86_64-unknown-linux-gnu`)
- ✅ **Best for**: Pure Rust logic, data validation, serialization
- ✅ **Advantages**: Fast execution, full debugging support, no browser dependencies
- ✅ **Use cases**: Unit tests, data structure validation, business logic

#### **WASM Testing** (`wasm-pack test`)
- ✅ **Best for**: Browser/Node.js specific functionality, JavaScript interop
- ✅ **Advantages**: Tests actual WASM execution environment
- ✅ **Use cases**: DOM interaction, WebGPU functionality, browser APIs

## Dependencies Setup

```toml
[dependencies]
# Core WASM dependencies
wasm-bindgen = "0.2"
serde = { version = "1.0", features = ["derive"] }
serde-wasm-bindgen = "0.6"

[dev-dependencies]
# Testing framework for WASM
wasm-bindgen-test = "0.3"
```

## Testing Commands

### **Native Testing**
```bash
# Run all native tests
cargo test --target x86_64-unknown-linux-gnu

# Run specific module tests
cargo test --target x86_64-unknown-linux-gnu store_state

# Run with output
cargo test --target x86_64-unknown-linux-gnu -- --nocapture
```

### **WASM Testing**
```bash
# Run tests in Node.js
wasm-pack test --node

# Run tests in Firefox headless
wasm-pack test --firefox --headless

# Run tests in Chrome headless
wasm-pack test --chrome --headless

# Run specific test file
wasm-pack test --firefox --headless --test store_state
```

## Test Organization

### **Unit Tests in Source Files**
```rust
// src/store_state.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation() {
        // Native test - runs with cargo test
    }

    #[wasm_bindgen_test]
    fn test_wasm_functionality() {
        // WASM test - runs with wasm-pack test
    }
}
```

### **Integration Tests**
```rust
// tests/integration_test.rs
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn test_full_integration() {
    // Integration test in browser environment
}
```

## Store Contract Testing Pattern

Based on our implementation, here's the proven pattern for testing store contracts:

### **1. Validation Testing**
```rust
#[test]
fn test_store_state_validation() {
    let store_state = StoreState {
        current_symbol: "BTC-USD".to_string(),
        // ... other fields
    };
    
    let validation = store_state.validate();
    assert!(validation.is_valid);
}
```

### **2. Serialization Testing**
```rust
#[test]
fn test_json_serialization() {
    let store_state = create_test_store_state();
    
    // Test serialization
    let json = serde_json::to_string(&store_state).expect("Failed to serialize");
    
    // Test deserialization
    let deserialized: StoreState = serde_json::from_str(&json).expect("Failed to deserialize");
    
    // Verify round trip
    assert_eq!(store_state.current_symbol, deserialized.current_symbol);
}
```

### **3. Field Name Compatibility Testing**
```rust
#[test]
fn test_camel_case_json_fields() {
    let config = ChartConfig {
        start_time: 1000,
        end_time: 2000,
        // ...
    };
    
    let json = serde_json::to_string(&config).expect("Failed to serialize");
    
    // Verify camelCase in JSON
    assert!(json.contains("\"startTime\""));
    assert!(json.contains("\"endTime\""));
    assert!(!json.contains("\"start_time\""));
}
```

## Common Issues and Solutions

### **Issue: Compilation Errors in Tests**
When the main library has compilation issues that prevent testing:

**Solution**: Create standalone test crates
```bash
cargo new --name test-module test_module
# Copy specific module code for isolated testing
```

### **Issue: WASM-specific Dependencies**
Some dependencies only work in WASM context.

**Solution**: Use conditional compilation
```rust
#[cfg(target_arch = "wasm32")]
use web_sys::Window;

#[cfg(not(target_arch = "wasm32"))]
fn mock_window_function() -> String {
    "mock".to_string()
}
```

### **Issue: Test Performance**
WASM tests are slower than native tests.

**Solution**: Use layered testing approach
1. Native tests for business logic (fast)
2. WASM tests for integration points (comprehensive)

## Testing Configuration

### **For CI/CD**
```bash
# In GitHub Actions
- name: Run Native Tests
  run: cargo test --target x86_64-unknown-linux-gnu

- name: Run WASM Tests  
  run: wasm-pack test --firefox --headless
```

### **For Development**
```bash
# Quick native tests during development
cargo test --target x86_64-unknown-linux-gnu

# Full WASM tests before commit
wasm-pack test --node --firefox --headless
```

## Best Practices

1. **Test Pure Logic Natively**: Use native tests for data validation, business logic
2. **Test Interop with WASM**: Use WASM tests for JavaScript bridge functionality
3. **Standalone Testing**: Create separate test crates for complex validation scenarios
4. **Field Name Testing**: Always verify JSON field naming matches TypeScript expectations
5. **Error Message Testing**: Ensure validation errors are helpful and specific
6. **Round Trip Testing**: Test serialization → deserialization for all data structures

## Success Metrics

- ✅ All native tests pass quickly (< 1 second)
- ✅ WASM tests verify browser compatibility
- ✅ JSON output matches TypeScript interface expectations
- ✅ Error messages are helpful for debugging
- ✅ Tests can run in CI/CD pipeline without browser installation complexity

This testing strategy ensures robust, maintainable WebAssembly applications with confidence in both Rust logic and JavaScript interop.