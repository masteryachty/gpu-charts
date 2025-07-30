#!/bin/bash

# Detailed test script for individual exchanges
cd /home/xander/projects/gpu-charts/logger

# Create local data directory
mkdir -p data/{okx,kraken,bitfinex}/{MD,TRADES}

echo "Starting detailed exchange testing..."
echo "======================================="

# Function to test an exchange
test_exchange() {
    local exchange=$1
    local duration=${2:-60}  # Default 60 seconds
    
    echo ""
    echo "=== Testing $exchange Exchange ==="
    echo "Start time: $(date)"
    
    # Clear previous data for clean test
    rm -f data/$exchange/MD/*.bin data/$exchange/TRADES/*.bin 2>/dev/null
    
    # Run the exchange for specified duration
    echo "Running $exchange for $duration seconds..."
    timeout ${duration}s cargo run --release -- run -e $exchange 2>&1 | tee ${exchange}-detailed-test.log &
    local pid=$!
    
    # Monitor data file creation
    local start_time=$(date +%s)
    local files_found=false
    
    while [ $(($(date +%s) - start_time)) -lt $duration ]; do
        sleep 5
        
        echo "[$exchange] Checking for data files at $(date +%H:%M:%S)..."
        
        # Check for MD files
        local md_files=$(find data/$exchange/MD -name "*.bin" -type f 2>/dev/null | wc -l)
        local trade_files=$(find data/$exchange/TRADES -name "*.bin" -type f 2>/dev/null | wc -l)
        
        if [ $md_files -gt 0 ] || [ $trade_files -gt 0 ]; then
            echo "[$exchange] Found $md_files MD files and $trade_files TRADE files"
            files_found=true
            
            # Check file sizes
            if [ $md_files -gt 0 ]; then
                echo "[$exchange] Market data files:"
                find data/$exchange/MD -name "*.bin" -type f -exec ls -lh {} \; 2>/dev/null | head -5
            fi
            
            if [ $trade_files -gt 0 ]; then
                echo "[$exchange] Trade files:"
                find data/$exchange/TRADES -name "*.bin" -type f -exec ls -lh {} \; 2>/dev/null | head -5
            fi
        fi
    done
    
    # Wait for process to finish
    wait $pid
    
    echo "[$exchange] Test completed at $(date)"
    
    # Final report
    echo "[$exchange] Final Report:"
    local final_md_files=$(find data/$exchange/MD -name "*.bin" -type f 2>/dev/null | wc -l)
    local final_trade_files=$(find data/$exchange/TRADES -name "*.bin" -type f 2>/dev/null | wc -l)
    
    echo "  - Market data files: $final_md_files"
    echo "  - Trade data files: $final_trade_files"
    
    if [ $final_md_files -gt 0 ] || [ $final_trade_files -gt 0 ]; then
        echo "  - Status: SUCCESS - Data is being recorded"
        
        # Show file sizes
        local total_size=$(du -sh data/$exchange 2>/dev/null | cut -f1)
        echo "  - Total data size: $total_size"
    else
        echo "  - Status: FAILED - No data files created"
        
        # Check logs for errors
        echo "  - Recent log entries:"
        tail -20 ${exchange}-detailed-test.log | grep -E "(ERROR|WARN|error|Error)" | head -10
    fi
    
    echo ""
}

# Test OKX with 90 seconds (might need more time to connect)
test_exchange "okx" 90

# Test Kraken with 60 seconds
test_exchange "kraken" 60

# Test Bitfinex with 60 seconds
test_exchange "bitfinex" 60

echo "======================================="
echo "All tests completed!"
echo ""
echo "Summary:"
echo "--------"

for exchange in okx kraken bitfinex; do
    local md_count=$(find data/$exchange/MD -name "*.bin" -type f 2>/dev/null | wc -l)
    local trade_count=$(find data/$exchange/TRADES -name "*.bin" -type f 2>/dev/null | wc -l)
    local status="FAILED"
    
    if [ $md_count -gt 0 ] || [ $trade_count -gt 0 ]; then
        status="SUCCESS"
    fi
    
    printf "%-10s: %s (MD: %d files, Trades: %d files)\n" "$exchange" "$status" "$md_count" "$trade_count"
done

echo ""
echo "Detailed logs available in:"
echo "  - okx-detailed-test.log"
echo "  - kraken-detailed-test.log"
echo "  - bitfinex-detailed-test.log"