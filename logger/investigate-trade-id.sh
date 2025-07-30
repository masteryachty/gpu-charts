#!/bin/bash

echo "=== Investigating Specific trade_id File Issue ==="
echo
echo "Since volume mapping is correct, investigating why only trade_id fails..."
echo

# Function to check inside Docker container
check_in_container() {
    local container_name="$1"
    
    echo "Checking inside container: $container_name"
    
    # Check if container is running
    if ! docker ps --format '{{.Names}}' | grep -q "$container_name"; then
        echo "Container $container_name is not running"
        return 1
    fi
    
    # Check file permissions inside container
    echo
    echo "File permissions in container:"
    docker exec "$container_name" find /mnt/md/data -name "id.*.bin" -ls 2>/dev/null | head -10
    
    # Check if specific to Binance
    echo
    echo "Binance trade_id files:"
    docker exec "$container_name" find /mnt/md/data/binance -name "id.*.bin" -path "*/TRADES/*" -ls 2>/dev/null
    
    # Check file handles
    echo
    echo "Open file handles for trade_id:"
    docker exec "$container_name" lsof 2>/dev/null | grep "id.*bin" | head -10
    
    # Check disk usage
    echo
    echo "Disk usage in container:"
    docker exec "$container_name" df -h /mnt/md/data
    
    # Test write permission
    echo
    echo "Testing write permission:"
    docker exec "$container_name" sh -c 'echo "test" > /mnt/md/data/test_write.tmp && echo "Write test: SUCCESS" && rm /mnt/md/data/test_write.tmp || echo "Write test: FAILED"'
}

# Function to check on host
check_on_host() {
    echo
    echo "=== Checking on Host System ==="
    
    # Find all id.*.bin files
    echo "All trade_id files on host:"
    find /mnt/HDDs/coinbase_logger -name "id.*.bin" -type f -ls 2>/dev/null | head -10
    
    # Check for corrupted files
    echo
    echo "Checking for zero-size or corrupted files:"
    find /mnt/HDDs/coinbase_logger -name "id.*.bin" -size 0 -ls 2>/dev/null
    
    # Check file system
    echo
    echo "File system check:"
    mount | grep "/mnt/HDDs/coinbase_logger"
    
    # Check for specific Binance trade_id issues
    echo
    echo "Binance-specific trade_id files:"
    find /mnt/HDDs/coinbase_logger/binance -name "id.*.bin" -path "*/TRADES/*" 2>/dev/null | while read -r file; do
        echo
        echo "File: $file"
        echo "Size: $(stat -c%s "$file" 2>/dev/null) bytes"
        echo "Permissions: $(stat -c%a "$file" 2>/dev/null)"
        echo "Last modified: $(stat -c%y "$file" 2>/dev/null)"
        
        # Try to read last 8 bytes (one trade_id record)
        if tail -c 8 "$file" >/dev/null 2>&1; then
            echo "Can read file: YES"
        else
            echo "Can read file: NO - may be corrupted"
        fi
    done
}

# Main execution
echo "Checking container name..."
CONTAINER_NAME=$(docker ps --format '{{.Names}}' | grep -E '(logger|exchange)' | head -1)

if [ -n "$CONTAINER_NAME" ]; then
    check_in_container "$CONTAINER_NAME"
else
    echo "No logger container found running"
fi

check_on_host

echo
echo "=== Possible Causes ==="
echo "1. File handle exhaustion - trade_id might be opening too many handles"
echo "2. Buffer size issue - u64 writes might need different buffering"
echo "3. Corrupted file preventing new writes"
echo "4. File system quota or inode limit reached"
echo
echo "=== Recommended Fix ==="
echo "1. Stop the container:"
echo "   docker stop $CONTAINER_NAME"
echo
echo "2. Remove corrupted trade_id files:"
echo "   find /mnt/HDDs/coinbase_logger/binance -name 'id.*.bin' -path '*/TRADES/*' -delete"
echo
echo "3. Restart container:"
echo "   docker start $CONTAINER_NAME"
echo
echo "4. Monitor for new errors:"
echo "   docker logs -f $CONTAINER_NAME 2>&1 | grep -E 'trade_id|flush'"