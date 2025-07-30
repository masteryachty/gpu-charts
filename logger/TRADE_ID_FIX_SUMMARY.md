# Trade ID File I/O Error Fix Summary

## Problem
The logger is experiencing I/O errors (os error 5) specifically with the `trade_id` file. The partial flush implementation shows that 14 out of 15 files flush successfully, with only `trade_id` failing.

## Root Cause
**Path Mismatch**: The Docker container expects data at `/mnt/md/data/` but TrueNAS has the data mounted at `/mnt/HDDs/coinbase_logger/`.

## Immediate Solution

### Fix Docker Volume Mapping
```bash
# Stop current container
docker stop <container_id>

# Run with correct volume mapping
docker run -d \
  --name multi-exchange-logger \
  -v /mnt/HDDs/coinbase_logger:/mnt/md/data \
  masteryachty/multi-exchange-logger:latest
```

### Or Use Docker Compose (Recommended)
```yaml
# docker-compose.yml
version: '3.8'

services:
  logger:
    image: masteryachty/multi-exchange-logger:latest
    container_name: multi-exchange-logger
    restart: unless-stopped
    volumes:
      - /mnt/HDDs/coinbase_logger:/mnt/md/data
    environment:
      - RUST_LOG=info
```

Then run:
```bash
docker-compose up -d
```

## Why Only trade_id File?
The `trade_id` file might be affected because:
1. It's written as a u64 (8 bytes) which might have different alignment requirements
2. It could be the first file in the flush order that encounters the path issue
3. The file might have been created with incorrect permissions during a previous failed attempt

## Verification Steps
1. Check if the issue is resolved:
   ```bash
   docker logs -f multi-exchange-logger 2>&1 | grep -E "flush|trade_id"
   ```

2. Look for successful flushes:
   ```
   # Should see: "Partial flush success: 15/15 files flushed"
   ```

3. Verify data is being written:
   ```bash
   ls -la /mnt/HDDs/coinbase_logger/binance/*/TRADES/id.*.bin
   ```

## Additional Fixes Implemented
1. **Retry Logic**: Now properly retries I/O errors up to 3 times with exponential backoff
2. **Partial Flush**: Continues operation even if individual files fail
3. **Better Logging**: Shows exactly which files fail to help diagnose issues

## Long-term Recommendations
1. Standardize mount paths across all deployments
2. Add mount point validation at startup
3. Implement health checks for file system access
4. Consider using environment variables for configurable paths