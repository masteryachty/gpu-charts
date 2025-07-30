#!/bin/bash

echo "=== Storage Setup Verification ==="
echo

# Check if running in Docker
if [ -f /.dockerenv ]; then
    echo "Running in Docker container"
    DATA_PATH="/mnt/md/data"
else
    echo "Running on host system"
    DATA_PATH="/mnt/HDDs/coinbase_logger"
fi

echo "Checking data path: $DATA_PATH"
echo

# Check if path exists
if [ -d "$DATA_PATH" ]; then
    echo "✓ Data path exists"
else
    echo "✗ Data path does not exist!"
    echo "  Creating directory structure..."
    mkdir -p "$DATA_PATH"
fi

# Check permissions
echo
echo "Permissions check:"
ls -la "$DATA_PATH" | head -5

# Check if writable
echo
echo "Write test:"
TEST_FILE="$DATA_PATH/test_write_$(date +%s).tmp"
if echo "test" > "$TEST_FILE" 2>/dev/null; then
    echo "✓ Write test successful"
    rm -f "$TEST_FILE"
else
    echo "✗ Write test failed!"
    echo "  Current user: $(whoami)"
    echo "  User ID: $(id -u)"
    echo "  Group ID: $(id -g)"
fi

# Check mount point
echo
echo "Mount information:"
mount | grep -E "(HDDs|md/data)" || echo "No relevant mounts found"

# Check disk space
echo
echo "Disk space:"
df -h "$DATA_PATH" 2>/dev/null || df -h /

# Check for existing data
echo
echo "Existing data check:"
if [ -d "$DATA_PATH/binance" ]; then
    echo "Binance data directory found:"
    find "$DATA_PATH/binance" -type f -name "*.bin" 2>/dev/null | wc -l
    echo "files found"
fi

# Docker volume mapping suggestion
echo
echo "=== Docker Run Command Suggestion ==="
echo "If running in Docker, use this volume mapping:"
echo "docker run -v /mnt/HDDs/coinbase_logger:/mnt/md/data ..."
echo
echo "Or update docker-compose.yml:"
echo "volumes:"
echo "  - /mnt/HDDs/coinbase_logger:/mnt/md/data"