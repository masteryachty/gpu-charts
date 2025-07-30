#!/bin/bash

# Test script for individual exchanges with local data path
cd /home/xander/projects/gpu-charts/logger

# Set local data path
export LOGGER__LOGGER__DATA_PATH="data"

# Create local data directory
mkdir -p data/{okx,kraken,bitfinex}/{MD,TRADES}

echo "Starting exchange testing with local data path..."
echo "======================================="

# Test OKX
echo ""
echo "=== Testing OKX Exchange ==="
echo "Start time: $(date)"

# Clear previous data for clean test
rm -f data/okx/MD/*.bin data/okx/TRADES/*.bin 2>/dev/null

# Run OKX for 60 seconds
timeout 60s cargo run --release -- run -e okx 2>&1 | tee okx-local-test.log &
OKX_PID=$!

sleep 20

echo "[OKX] Checking for data files..."
OKX_MD=$(find data/okx -name "*.bin" -type f | grep MD | wc -l)
OKX_TRADES=$(find data/okx -name "*.bin" -type f | grep TRADES | wc -l)
echo "[OKX] Found $OKX_MD MD files and $OKX_TRADES trade files"

wait $OKX_PID

# Test Kraken
echo ""
echo "=== Testing Kraken Exchange ==="
echo "Start time: $(date)"

# Clear previous data for clean test
rm -f data/kraken/MD/*.bin data/kraken/TRADES/*.bin 2>/dev/null

# Run Kraken for 60 seconds
timeout 60s cargo run --release -- run -e kraken 2>&1 | tee kraken-local-test.log &
KRAKEN_PID=$!

sleep 20

echo "[Kraken] Checking for data files..."
KRAKEN_MD=$(find data/kraken -name "*.bin" -type f | grep MD | wc -l)
KRAKEN_TRADES=$(find data/kraken -name "*.bin" -type f | grep TRADES | wc -l)
echo "[Kraken] Found $KRAKEN_MD MD files and $KRAKEN_TRADES trade files"

wait $KRAKEN_PID

# Test Bitfinex
echo ""
echo "=== Testing Bitfinex Exchange ==="
echo "Start time: $(date)"

# Clear previous data for clean test
rm -f data/bitfinex/MD/*.bin data/bitfinex/TRADES/*.bin 2>/dev/null

# Run Bitfinex for 60 seconds
timeout 60s cargo run --release -- run -e bitfinex 2>&1 | tee bitfinex-local-test.log &
BITFINEX_PID=$!

sleep 20

echo "[Bitfinex] Checking for data files..."
BITFINEX_MD=$(find data/bitfinex -name "*.bin" -type f | grep MD | wc -l)
BITFINEX_TRADES=$(find data/bitfinex -name "*.bin" -type f | grep TRADES | wc -l)
echo "[Bitfinex] Found $BITFINEX_MD MD files and $BITFINEX_TRADES trade files"

wait $BITFINEX_PID

echo ""
echo "======================================="
echo "Test Summary:"
echo "======================================="
echo ""

# Final check
OKX_FINAL_MD=$(find data/okx -name "*.bin" -type f | grep MD | wc -l)
OKX_FINAL_TRADES=$(find data/okx -name "*.bin" -type f | grep TRADES | wc -l)
KRAKEN_FINAL_MD=$(find data/kraken -name "*.bin" -type f | grep MD | wc -l)
KRAKEN_FINAL_TRADES=$(find data/kraken -name "*.bin" -type f | grep TRADES | wc -l)
BITFINEX_FINAL_MD=$(find data/bitfinex -name "*.bin" -type f | grep MD | wc -l)
BITFINEX_FINAL_TRADES=$(find data/bitfinex -name "*.bin" -type f | grep TRADES | wc -l)

printf "%-10s: MD=%4d files, Trades=%4d files\n" "OKX" "$OKX_FINAL_MD" "$OKX_FINAL_TRADES"
printf "%-10s: MD=%4d files, Trades=%4d files\n" "Kraken" "$KRAKEN_FINAL_MD" "$KRAKEN_FINAL_TRADES"
printf "%-10s: MD=%4d files, Trades=%4d files\n" "Bitfinex" "$BITFINEX_FINAL_MD" "$BITFINEX_FINAL_TRADES"

echo ""
echo "Detailed logs available in:"
echo "  - okx-local-test.log"
echo "  - kraken-local-test.log"
echo "  - bitfinex-local-test.log"