#!/bin/bash

# Test the multi-exchange logger

echo "Building the logger..."
cargo build -p logger --target x86_64-unknown-linux-gnu --release

echo ""
echo "Testing exchange connections..."
echo "1. Testing Coinbase..."
cargo run -p logger --target x86_64-unknown-linux-gnu -- test coinbase

echo ""
echo "2. Testing Binance..."
cargo run -p logger --target x86_64-unknown-linux-gnu -- test binance

echo ""
echo "Listing symbols from exchanges..."
echo "1. Coinbase symbols (first 10):"
cargo run -p logger --target x86_64-unknown-linux-gnu -- symbols coinbase 2>/dev/null | head -11

echo ""
echo "2. Binance symbols (first 10):"
cargo run -p logger --target x86_64-unknown-linux-gnu -- symbols binance 2>/dev/null | head -11

echo ""
echo "Test completed!"