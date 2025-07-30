#!/bin/bash

echo "=== Comprehensive Trade ID File Fix ==="
echo
echo "This script will diagnose and fix the trade_id file I/O error"
echo

# Function to check and fix files
fix_trade_id_files() {
    local base_path="$1"
    echo "Checking path: $base_path"
    
    # Find all trade id files
    echo "Finding all trade id files..."
    find "$base_path" -name "id.*.bin" -path "*/TRADES/*" 2>/dev/null | while read -r file; do
        echo
        echo "Found: $file"
        
        # Check if file is accessible
        if [ -r "$file" ]; then
            echo "  ✓ File is readable"
            
            # Check size
            size=$(stat -c%s "$file" 2>/dev/null || stat -f%z "$file" 2>/dev/null)
            echo "  Size: $size bytes"
            
            # Try to read first few bytes
            if head -c 8 "$file" >/dev/null 2>&1; then
                echo "  ✓ Can read file contents"
            else
                echo "  ✗ Cannot read file contents - file may be corrupted"
                echo "  Attempting to fix..."
                
                # Option 1: Try to copy the file
                if cp "$file" "${file}.backup" 2>/dev/null; then
                    echo "  ✓ Created backup: ${file}.backup"
                    # Remove original and rename backup
                    rm -f "$file" && mv "${file}.backup" "$file"
                    echo "  ✓ Replaced file with backup"
                else
                    echo "  ✗ Cannot create backup - removing corrupted file"
                    rm -f "$file"
                    echo "  ✓ Removed corrupted file (will be recreated on next write)"
                fi
            fi
        else
            echo "  ✗ File is not readable - removing"
            rm -f "$file" 2>/dev/null
            echo "  ✓ Removed inaccessible file"
        fi
    done
}

# Check both possible paths
echo "=== Checking TrueNAS path ==="
if [ -d "/mnt/HDDs/coinbase_logger" ]; then
    fix_trade_id_files "/mnt/HDDs/coinbase_logger"
else
    echo "TrueNAS path not found"
fi

echo
echo "=== Checking container path ==="
if [ -d "/mnt/md/data" ]; then
    fix_trade_id_files "/mnt/md/data"
else
    echo "Container path not found"
fi

echo
echo "=== Recommended Actions ==="
echo
echo "1. IMMEDIATE FIX - Update Docker volume mapping:"
echo "   docker run -v /mnt/HDDs/coinbase_logger:/mnt/md/data masteryachty/multi-exchange-logger:latest"
echo
echo "2. Check for file system errors:"
echo "   dmesg | grep -i error | tail -20"
echo
echo "3. If errors persist, check disk health:"
echo "   smartctl -a /dev/sdX  # Replace X with your disk"
echo
echo "4. Clear Docker volumes and restart:"
echo "   docker stop <container_id>"
echo "   docker rm <container_id>"
echo "   docker run -v /mnt/HDDs/coinbase_logger:/mnt/md/data masteryachty/multi-exchange-logger:latest"
echo
echo "5. Monitor logs after restart:"
echo "   docker logs -f <container_id> 2>&1 | grep -E 'trade_id|flush'"