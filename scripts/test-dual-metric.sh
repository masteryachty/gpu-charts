#!/bin/bash

# Test script for dual-metric rendering functionality
# Run all dual-metric related tests

set -e

echo "🧪 Running Dual-Metric Test Suite"
echo "=================================="

# Change to project root
cd "$(dirname "$0")/.."

echo ""
echo "1️⃣ Running WASM tests for dual-metric functionality..."
cd charting
wasm-pack test --node --release -- --features="test" tests/dual_metric_tests.rs

echo ""
echo "2️⃣ Running React unit tests for dual-metric store..."
cd ../web
npm run test -- tests/unit/dual-metric-store.spec.ts

echo ""
echo "3️⃣ Running React unit tests for store validation..."
npm run test -- tests/unit/store-validation.spec.ts

echo ""
echo "4️⃣ Running integration tests for dual-metric UI..."
npm run test -- tests/integration/chart-controls-metrics.spec.ts

echo ""
echo "5️⃣ Running integration tests for dual-metric rendering..."
npm run test -- tests/integration/dual-metric-ui.spec.ts

echo ""
echo "6️⃣ Running WASM bridge integration tests..."
npm run test -- tests/integration/dual-metric-wasm-bridge.spec.ts

echo ""
echo "✅ All dual-metric tests completed!"
echo ""
echo "📊 Test Summary:"
echo "- WASM color generation and URL construction"
echo "- React store metrics management" 
echo "- UI controls for metric selection"
echo "- Integration between React and WASM"
echo "- Error handling and edge cases"
echo "- Performance and validation"