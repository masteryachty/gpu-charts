#!/bin/bash

# API Integration Test Script

set -e

echo "🧪 Starting API Server Integration Tests"

# Check if server is running
if ! curl -k -s "https://localhost:8443/api/symbols" > /dev/null 2>&1; then
    echo "❌ Server is not running on https://localhost:8443"
    echo "   Please start the server with: cargo run --target x86_64-unknown-linux-gnu"
    exit 1
fi

echo "✅ Server is running"

# Test 1: Symbols endpoint
echo "🔍 Testing /api/symbols endpoint..."
SYMBOLS_RESPONSE=$(curl -k -s "https://localhost:8443/api/symbols")
if echo "$SYMBOLS_RESPONSE" | grep -q '"symbols"'; then
    SYMBOL_COUNT=$(echo "$SYMBOLS_RESPONSE" | grep -o '"[^"]*"' | wc -l)
    echo "✅ Symbols endpoint returned $((SYMBOL_COUNT - 1)) symbols"
else
    echo "❌ Symbols endpoint failed"
    echo "Response: $SYMBOLS_RESPONSE"
    exit 1
fi

# Test 2: Data endpoint with valid parameters
echo "🔍 Testing /api/data endpoint with valid parameters..."
DATA_RESPONSE=$(curl -k -s "https://localhost:8443/api/data?symbol=BTC-USD&type=MD&start=1745322750&end=1745391150&columns=time,best_bid" | head -c 200)
if echo "$DATA_RESPONSE" | grep -q '"columns"'; then
    echo "✅ Data endpoint returned valid response"
    echo "Response preview: $(echo "$DATA_RESPONSE" | head -c 100)..."
else
    echo "❌ Data endpoint failed"
    echo "Response: $DATA_RESPONSE"
    exit 1
fi

# Test 3: Data endpoint with missing parameters
echo "🔍 Testing /api/data endpoint with missing parameters..."
ERROR_RESPONSE=$(curl -k -s "https://localhost:8443/api/data?symbol=BTC-USD&type=MD")
if echo "$ERROR_RESPONSE" | grep -q -i "missing\|error"; then
    echo "✅ Data endpoint correctly rejected invalid request"
else
    echo "❌ Data endpoint should have returned error"
    echo "Response: $ERROR_RESPONSE"
    exit 1
fi

# Test 4: Invalid endpoint
echo "🔍 Testing invalid endpoint..."
NOT_FOUND_RESPONSE=$(curl -k -s -w "%{http_code}" "https://localhost:8443/api/invalid" -o /dev/null)
if [ "$NOT_FOUND_RESPONSE" = "404" ]; then
    echo "✅ Invalid endpoint correctly returned 404"
else
    echo "❌ Invalid endpoint should return 404, got $NOT_FOUND_RESPONSE"
    exit 1
fi

# Test 5: CORS headers
echo "🔍 Testing CORS headers..."
CORS_RESPONSE=$(curl -k -s -H "Origin: http://localhost:3000" -I "https://localhost:8443/api/symbols")
if echo "$CORS_RESPONSE" | grep -q "Access-Control-Allow-Origin"; then
    echo "✅ CORS headers present"
else
    echo "❌ CORS headers missing"
    echo "Response: $CORS_RESPONSE"
    exit 1
fi

# Test 6: OPTIONS request (preflight)
echo "🔍 Testing OPTIONS preflight request..."
OPTIONS_RESPONSE=$(curl -k -s -X OPTIONS -H "Origin: http://localhost:3000" -w "%{http_code}" "https://localhost:8443/api/data" -o /dev/null)
if [ "$OPTIONS_RESPONSE" = "200" ]; then
    echo "✅ OPTIONS request handled correctly"
else
    echo "❌ OPTIONS request failed, got $OPTIONS_RESPONSE"
    exit 1
fi

echo ""
echo "🎉 All API tests passed!"
echo "📊 Test Summary:"
echo "   ✅ Symbols endpoint"
echo "   ✅ Data endpoint (valid request)"
echo "   ✅ Data endpoint (error handling)"
echo "   ✅ Invalid endpoint (404)"
echo "   ✅ CORS headers"
echo "   ✅ OPTIONS preflight"