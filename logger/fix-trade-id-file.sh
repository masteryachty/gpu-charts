#!/bin/bash

echo "=== Investigating trade_id File Issue ==="
echo

# Set the data path based on environment
if [ -f /.dockerenv ]; then
    DATA_PATH="/mnt/md/data"
else
    DATA_PATH="/mnt/HDDs/coinbase_logger"
fi

echo "Data path: $DATA_PATH"
echo

# Find all trade_id files
echo "Looking for trade_id files..."
find "$DATA_PATH" -name "trade_id.*" -type f 2>/dev/null | while read -r file; do
    echo
    echo "Found: $file"
    echo "Size: $(ls -lh "$file" | awk '{print $5}')"
    echo "Permissions: $(ls -l "$file" | awk '{print $1}')"
    echo "Owner: $(ls -l "$file" | awk '{print $3":"$4}')"
    echo "Last modified: $(stat -c %y "$file" 2>/dev/null || stat -f %Sm "$file" 2>/dev/null)"
    
    # Check if file is locked
    if lsof "$file" >/dev/null 2>&1; then
        echo "File is currently open by:"
        lsof "$file"
    fi
    
    # Check file integrity
    if [ -r "$file" ]; then
        echo "File is readable"
    else
        echo "WARNING: File is not readable!"
    fi
    
    if [ -w "$file" ]; then
        echo "File is writable"
    else
        echo "WARNING: File is not writable!"
    fi
done

echo
echo "=== Checking for Binance trade_id files specifically ==="
BINANCE_TRADE_PATH="$DATA_PATH/binance"

if [ -d "$BINANCE_TRADE_PATH" ]; then
    echo "Binance trade files:"
    find "$BINANCE_TRADE_PATH" -path "*/TRADES/trade_id.*" -type f -ls 2>/dev/null
else
    echo "Binance directory not found at $BINANCE_TRADE_PATH"
fi

echo
echo "=== Checking disk for bad sectors ==="
# Check if any I/O errors in system logs
echo "Recent I/O errors in system log:"
dmesg | grep -i "i/o error" | tail -5

echo
echo "=== Fix Suggestions ==="
echo "1. Remove the problematic file (data will be lost for this file only):"
echo "   find $DATA_PATH -name 'trade_id.*' -path '*/binance/*' -exec rm {} \;"
echo
echo "2. Try to copy the file to test readability:"
echo "   find $DATA_PATH -name 'trade_id.*' -path '*/binance/*' -exec cp {} {}.backup \;"
echo
echo "3. Check file system:"
echo "   fsck -n $DATA_PATH  # Dry run only"
echo
echo "4. Restart the logger to create new file handles"