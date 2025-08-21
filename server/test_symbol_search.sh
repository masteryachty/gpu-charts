#!/bin/bash

# Test script for the symbol search endpoint
# This script demonstrates various search queries and expected behavior

echo "Symbol Search Endpoint Test Cases"
echo "================================="
echo ""

# Test cases for symbol search
echo "Test 1: Search for 'btc' (should match Bitcoin pairs)"
echo "Expected: BTC/USD from multiple exchanges"
echo "curl -k 'https://localhost:8443/api/symbol-search?q=btc'"
echo ""

echo "Test 2: Search for 'bitcoin' (should match by display name)"
echo "Expected: BTC/USD with Bitcoin in display name"
echo "curl -k 'https://localhost:8443/api/symbol-search?q=bitcoin'"
echo ""

echo "Test 3: Search for 'eth' (should match Ethereum)"
echo "Expected: ETH/USD from multiple exchanges"
echo "curl -k 'https://localhost:8443/api/symbol-search?q=eth'"
echo ""

echo "Test 4: Search for 'usd' (should match USD pairs)"
echo "Expected: Multiple pairs with USD as quote currency"
echo "curl -k 'https://localhost:8443/api/symbol-search?q=usd'"
echo ""

echo "Test 5: Search for 'doge' (should match Dogecoin)"
echo "Expected: DOGE/USD from multiple exchanges"
echo "curl -k 'https://localhost:8443/api/symbol-search?q=doge'"
echo ""

echo "Test 6: Search for 'layer2' (should match by tag)"
echo "Expected: ARB/USD and OP/USD (Layer 2 solutions)"
echo "curl -k 'https://localhost:8443/api/symbol-search?q=layer2'"
echo ""

echo "Test 7: Search for 'defi' (should match DeFi tokens)"
echo "Expected: UNI/USD (Uniswap)"
echo "curl -k 'https://localhost:8443/api/symbol-search?q=defi'"
echo ""

echo "Test 8: Search for partial match 'sol'"
echo "Expected: SOL/USD (Solana)"
echo "curl -k 'https://localhost:8443/api/symbol-search?q=sol'"
echo ""

echo "Test 9: Empty search query"
echo "Expected: Empty results array"
echo "curl -k 'https://localhost:8443/api/symbol-search?q='"
echo ""

echo "Test 10: Case insensitive search 'BTC'"
echo "Expected: Same results as lowercase 'btc'"
echo "curl -k 'https://localhost:8443/api/symbol-search?q=BTC'"
echo ""

echo "================================="
echo "To run these tests, start the server with:"
echo "cd server && cargo run --target x86_64-unknown-linux-gnu"
echo "Then execute the curl commands above"