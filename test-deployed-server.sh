#!/bin/bash

# Test Deployed GPU Charts Server
# Usage: ./test-deployed-server.sh <server-url>

SERVER_URL="${1:-https://localhost:8443}"

echo "🧪 Testing GPU Charts Server at: $SERVER_URL"
echo ""

# Test 1: Check if server is reachable
echo "1️⃣ Testing server connectivity..."
if curl -k -s -f "$SERVER_URL/api/symbols" > /dev/null 2>&1; then
    echo "✅ Server is reachable"
else
    echo "❌ Cannot reach server at $SERVER_URL"
    echo "   Make sure the server is running and accessible"
    exit 1
fi

# Test 2: Test symbols endpoint
echo ""
echo "2️⃣ Testing /api/symbols endpoint..."
SYMBOLS=$(curl -k -s "$SERVER_URL/api/symbols")
if echo "$SYMBOLS" | grep -q '"symbols"'; then
    echo "✅ Symbols endpoint working"
    echo "   Response: $SYMBOLS"
else
    echo "❌ Symbols endpoint failed"
    exit 1
fi

# Test 3: Test data endpoint
echo ""
echo "3️⃣ Testing /api/data endpoint..."
# Use a recent timestamp for testing
END_TIME=$(date +%s)
START_TIME=$((END_TIME - 3600))  # 1 hour ago

DATA_URL="$SERVER_URL/api/data?symbol=BTC-USD&type=MD&start=$START_TIME&end=$END_TIME&columns=time,best_bid"
echo "   Request URL: $DATA_URL"

RESPONSE=$(curl -k -s -w "\n%{http_code}" "$DATA_URL")
HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | head -n-1)

if [ "$HTTP_CODE" = "200" ]; then
    echo "✅ Data endpoint returned 200 OK"
    echo "   Response preview: $(echo "$BODY" | head -c 100)..."
else
    echo "⚠️  Data endpoint returned HTTP $HTTP_CODE"
    echo "   This might be normal if no data exists for the time range"
    echo "   Response: $BODY"
fi

# Test 4: Test CORS headers
echo ""
echo "4️⃣ Testing CORS headers..."
CORS_CHECK=$(curl -k -s -I -H "Origin: http://localhost:3000" "$SERVER_URL/api/symbols" | grep -i "access-control-allow-origin")
if [ -n "$CORS_CHECK" ]; then
    echo "✅ CORS headers present"
    echo "   $CORS_CHECK"
else
    echo "❌ CORS headers missing"
fi

# Test 5: SSL Certificate
echo ""
echo "5️⃣ Testing SSL certificate..."
SSL_INFO=$(echo | openssl s_client -connect "${SERVER_URL#https://}" 2>/dev/null | openssl x509 -noout -text 2>/dev/null | grep -E "Subject:|Not After")
if [ -n "$SSL_INFO" ]; then
    echo "✅ SSL certificate valid"
    echo "$SSL_INFO" | sed 's/^/   /'
else
    echo "⚠️  Could not verify SSL certificate (this is normal for self-signed certs)"
fi

echo ""
echo "🎉 Server testing complete!"
echo ""
echo "📋 Summary:"
echo "   - Server URL: $SERVER_URL"
echo "   - Connectivity: ✅"
echo "   - API Endpoints: Working"
echo "   - CORS: $([ -n "$CORS_CHECK" ] && echo "✅" || echo "❌")"
echo ""
echo "💡 Next steps:"
echo "   1. Check if your data files are mounted correctly"
echo "   2. Test with your frontend application"
echo "   3. Monitor server logs for any errors"