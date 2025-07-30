#!/bin/bash

# Test script to verify flush behavior with I/O errors
cd /home/xander/projects/gpu-charts/logger

echo "Testing flush behavior with local data directory..."
echo "================================================"

# Create test data directory
mkdir -p test-data

# Run with a short flush interval to test quickly
LOGGER__LOGGER__DATA_PATH="test-data" \
LOGGER__LOGGER__FLUSH_INTERVAL_SECS="2" \
timeout 10s cargo run --release -- -c config-local.yaml run -e binance 2>&1 | tee flush-test.log

echo ""
echo "Checking for flush errors in log..."
grep -E "Failed to flush|I/O error" flush-test.log || echo "No flush errors found - working correctly!"

echo ""
echo "Checking data files created..."
find test-data -name "*.bin" -type f | wc -l

# Clean up
rm -rf test-data
rm -f flush-test.log