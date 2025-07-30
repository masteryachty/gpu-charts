#!/bin/bash

# Test script for individual exchanges
cd /home/xander/projects/gpu-charts/logger

# Create local data directory
mkdir -p data

# Test OKX
echo "=== Testing OKX Exchange ==="
timeout 30s cargo run -- run -e okx 2>&1 | tee okx-test.log
echo "OKX data files:"
find data/okx -type f 2>/dev/null | head -20
echo ""

# Test Kraken  
echo "=== Testing Kraken Exchange ==="
timeout 30s cargo run -- run -e kraken 2>&1 | tee kraken-test.log
echo "Kraken data files:"
find data/kraken -type f 2>/dev/null | head -20
echo ""

# Test Bitfinex
echo "=== Testing Bitfinex Exchange ==="
timeout 30s cargo run -- run -e bitfinex 2>&1 | tee bitfinex-test.log
echo "Bitfinex data files:"
find data/bitfinex -type f 2>/dev/null | head -20
echo ""

echo "=== Test Complete ==="
echo "Check the log files for detailed output:"
echo "- okx-test.log"
echo "- kraken-test.log" 
echo "- bitfinex-test.log"