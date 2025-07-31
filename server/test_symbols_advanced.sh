#!/bin/bash

# Advanced test script for the /api/symbols endpoint

echo "Advanced /api/symbols Tests"
echo "==========================="
echo ""

# Function to pretty print a test
test_api() {
    local description="$1"
    local endpoint="$2"
    
    echo "Test: $description"
    echo "Endpoint: $endpoint"
    echo "---"
    
    response=$(curl -k -s "$endpoint")
    
    if [ $? -eq 0 ]; then
        # Check if response is valid JSON
        if echo "$response" | jq -e . > /dev/null 2>&1; then
            echo "✓ Valid JSON response"
            
            # Show summary based on the test
            if [[ "$endpoint" == *"exchange="* ]]; then
                # Single exchange filter
                exchange=$(echo "$endpoint" | sed -n 's/.*exchange=\([^&]*\).*/\1/p')
                symbol_count=$(echo "$response" | jq ".exchanges.$exchange // [] | length" 2>/dev/null)
                echo "✓ Found $symbol_count symbols for $exchange"
                
                # Show first 3 symbols
                echo "  First 3 symbols (newest first):"
                echo "$response" | jq -r ".exchanges.$exchange // [] | .[:3][] | \"    - \(.symbol) (updated: \(.last_update_date))\"" 2>/dev/null
            else
                # All exchanges
                exchange_count=$(echo "$response" | jq '.exchanges | keys | length' 2>/dev/null)
                total_symbols=$(echo "$response" | jq '.symbols | length' 2>/dev/null)
                echo "✓ Found $exchange_count exchanges with $total_symbols unique symbols"
                
                # Show summary per exchange
                echo "  Symbols per exchange:"
                echo "$response" | jq -r '.exchanges | to_entries[] | "    - \(.key): \(.value | length) symbols"' 2>/dev/null
            fi
        else
            echo "✗ Invalid JSON response"
            echo "$response"
        fi
    else
        echo "✗ Failed to connect to server"
    fi
    
    echo ""
}

# Run tests
test_api "Get all symbols from all exchanges" "https://localhost:8443/api/symbols"

test_api "Filter by Coinbase exchange" "https://localhost:8443/api/symbols?exchange=coinbase"

test_api "Filter by Binance exchange" "https://localhost:8443/api/symbols?exchange=binance"

test_api "Filter by Kraken exchange" "https://localhost:8443/api/symbols?exchange=kraken"

test_api "Filter by OKX exchange" "https://localhost:8443/api/symbols?exchange=okx"

test_api "Filter by Bitfinex exchange" "https://localhost:8443/api/symbols?exchange=bitfinex"

# Test invalid exchange
test_api "Test invalid exchange filter" "https://localhost:8443/api/symbols?exchange=invalid"

# Show recently updated symbols across all exchanges
echo "Recently Updated Symbols (Last Hour)"
echo "-----------------------------------"
response=$(curl -k -s "https://localhost:8443/api/symbols")
if echo "$response" | jq -e . > /dev/null 2>&1; then
    cutoff=$(($(date +%s) - 3600))
    echo "$response" | jq -r --arg cutoff "$cutoff" '
        .exchanges | to_entries[] | 
        {
            exchange: .key,
            recent: [.value[] | select(.last_update > ($cutoff | tonumber))]
        } | 
        select(.recent | length > 0) |
        "\(.exchange):\n" + (.recent | map("  - \(.symbol) (updated: \(.last_update_date))") | join("\n"))
    ' 2>/dev/null
    
    if [ $? -ne 0 ] || [ -z "$(echo "$response" | jq -r --arg cutoff "$cutoff" '.exchanges | to_entries[] | .value[] | select(.last_update > ($cutoff | tonumber)) | .symbol' 2>/dev/null)" ]; then
        echo "No symbols updated in the last hour"
    fi
fi