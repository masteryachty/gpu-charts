#!/bin/bash

# Test script for the enhanced /api/symbols endpoint

echo "Testing /api/symbols endpoint..."
echo "================================"

# Check if an exchange filter was provided as argument
if [ ! -z "$1" ]; then
    echo "Filtering by exchange: $1"
    endpoint="https://localhost:8443/api/symbols?exchange=$1"
else
    endpoint="https://localhost:8443/api/symbols"
fi

# Test the symbols endpoint
response=$(curl -k -s "$endpoint")

if [ $? -eq 0 ]; then
    echo "✓ Successfully connected to server"
    echo ""
    echo "Response:"
    echo "$response" | jq '.' 2>/dev/null || echo "$response"
    echo ""
    
    # Check if we have the expected structure
    if echo "$response" | jq -e '.symbols' > /dev/null 2>&1; then
        echo "✓ Found 'symbols' array"
        symbol_count=$(echo "$response" | jq '.symbols | length')
        echo "  Total unique symbols: $symbol_count"
    fi
    
    if echo "$response" | jq -e '.exchanges' > /dev/null 2>&1; then
        echo "✓ Found 'exchanges' object"
        echo ""
        echo "Exchanges and their symbols (sorted by newest first):"
        echo "$response" | jq -r '.exchanges | to_entries[] | "\n\(.key):"' | while read -r exchange; do
            if [ ! -z "$exchange" ] && [ "$exchange" != ":" ]; then
                echo "$exchange"
                exchange_name=$(echo "$exchange" | sed 's/://')
                echo "$response" | jq -r ".exchanges.\"$exchange_name\"[] | \"  - \(.symbol) (last update: \(.last_update_date))\"" 2>/dev/null
            fi
        done
    fi
else
    echo "✗ Failed to connect to server"
    echo "  Make sure the server is running on https://localhost:8443"
fi