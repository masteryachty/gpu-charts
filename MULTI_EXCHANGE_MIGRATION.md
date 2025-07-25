# Multi-Exchange Logger Migration Guide

## Overview

This guide covers the migration from the single-exchange `coinbase-logger` to the new multi-exchange `logger` system. The new system supports multiple cryptocurrency exchanges (initially Coinbase and Binance) with a unified data format and improved architecture.

## Key Changes

### 1. Directory Structure Changes

#### Old Structure
```
/mnt/md/data/
├── {symbol}/          # e.g., BTC-USD/
│   └── {type}/        # e.g., MD/
│       ├── time.{DD}.{MM}.{YY}.bin
│       ├── best_bid.{DD}.{MM}.{YY}.bin
│       └── ...
```

#### New Structure
```
/mnt/md/data/
├── {exchange}/        # e.g., coinbase/, binance/
│   ├── {symbol}/      # e.g., BTC-USD/, BTC-USDT/
│   │   └── {type}/    # e.g., MD/, TRADES/
│   │       ├── time.{DD}.{MM}.{YY}.bin
│   │       ├── best_bid.{DD}.{MM}.{YY}.bin
│   │       └── ...
```

### 2. Application Changes

- **Old**: `coinbase-logger` - Single exchange support only
- **New**: `logger` - Multi-exchange support with modular architecture

### 3. Configuration Changes

#### Old Configuration (implicit)
- Hardcoded Coinbase endpoints
- Fixed connection parameters
- No symbol mapping

#### New Configuration (`config.yaml`)
```yaml
logger:
  data_path: "/mnt/md/data"
  buffer_size: 8192
  flush_interval_secs: 5
  health_check_port: 8080

exchanges:
  coinbase:
    enabled: true
    ws_endpoint: "wss://ws-feed.exchange.coinbase.com"
    rest_endpoint: "https://api.exchange.coinbase.com"
    max_connections: 10
    symbols_per_connection: 50
    
  binance:
    enabled: true
    ws_endpoint: "wss://stream.binance.com:9443"
    rest_endpoint: "https://api.binance.com"
    max_connections: 5
    symbols_per_connection: 100
    ping_interval_secs: 20

symbol_mappings:
  mappings_file: "symbol_mappings.yaml"
  auto_discover: true
```

### 4. Data Server API Changes

#### Updated Data Endpoint
The `/api/data` endpoint now accepts an optional `exchange` parameter:

**Old Request**:
```
GET /api/data?symbol=BTC-USD&type=MD&start=123&end=456&columns=time,best_bid
```

**New Request** (with exchange):
```
GET /api/data?exchange=coinbase&symbol=BTC-USD&type=MD&start=123&end=456&columns=time,best_bid
```

**Backward Compatibility**: If `exchange` is omitted, it defaults to `coinbase`.

#### Updated Symbols Endpoint
The `/api/symbols` endpoint now returns symbols grouped by exchange:

**Old Response**:
```json
{
  "symbols": ["BTC-USD", "ETH-USD", "SOL-USD"]
}
```

**New Response**:
```json
{
  "symbols": ["BTC-USD", "ETH-USD", "SOL-USD", "BTC-USDT", "ETH-USDT"],
  "exchanges": {
    "coinbase": ["BTC-USD", "ETH-USD", "SOL-USD"],
    "binance": ["BTC-USDT", "ETH-USDT", "SOL-USDT"]
  }
}
```

## Migration Steps

### Phase 1: Preparation (Before Migration)

1. **Backup Existing Data**
   ```bash
   # Create backup of existing data
   sudo cp -r /mnt/md/data /mnt/md/data.backup
   ```

2. **Stop Current Logger**
   ```bash
   # Stop coinbase-logger service
   sudo systemctl stop coinbase-logger
   ```

### Phase 2: Data Migration

1. **Create New Directory Structure**
   ```bash
   # Create exchange directories
   sudo mkdir -p /mnt/md/data/coinbase
   ```

2. **Move Existing Data**
   ```bash
   # Move all existing symbol directories under coinbase/
   cd /mnt/md/data
   for symbol in */; do
     if [ "$symbol" != "coinbase/" ]; then
       sudo mv "$symbol" "coinbase/$symbol"
     fi
   done
   ```

3. **Verify Migration**
   ```bash
   # Check new structure
   ls -la /mnt/md/data/coinbase/
   # Should show: BTC-USD/, ETH-USD/, etc.
   ```

### Phase 3: Deploy New Logger

1. **Build New Logger**
   ```bash
   cd logger
   cargo build --release --target x86_64-unknown-linux-gnu
   ```

2. **Create Configuration Files**
   ```bash
   # Copy default configs
   cp config.yaml /etc/logger/
   cp symbol_mappings.yaml /etc/logger/
   ```

3. **Update Service File**
   Create `/etc/systemd/system/logger.service`:
   ```ini
   [Unit]
   Description=Multi-Exchange Cryptocurrency Logger
   After=network.target

   [Service]
   Type=simple
   User=logger
   Group=logger
   ExecStart=/usr/local/bin/logger
   Restart=always
   RestartSec=5
   Environment="RUST_LOG=info"
   WorkingDirectory=/etc/logger

   [Install]
   WantedBy=multi-user.target
   ```

4. **Start New Logger**
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable logger
   sudo systemctl start logger
   ```

### Phase 4: Update Server

1. **Deploy Updated Server**
   The server has been updated to handle the new file structure with backward compatibility.

2. **Test API Endpoints**
   ```bash
   # Test symbols endpoint
   curl -k "https://localhost:8443/api/symbols"
   
   # Test data endpoint (backward compatible)
   curl -k "https://localhost:8443/api/data?symbol=BTC-USD&type=MD&start=1234567890&end=1234567900&columns=time,best_bid"
   
   # Test data endpoint (with exchange)
   curl -k "https://localhost:8443/api/data?exchange=coinbase&symbol=BTC-USD&type=MD&start=1234567890&end=1234567900&columns=time,best_bid"
   ```

### Phase 5: Web Application Updates

Update frontend code to handle multiple exchanges:

1. **Update API calls to include exchange parameter**
2. **Update symbol selection to show exchange options**
3. **Handle new symbols response format**

## Rollback Plan

If issues arise during migration:

1. **Stop New Logger**
   ```bash
   sudo systemctl stop logger
   ```

2. **Restore Data Structure**
   ```bash
   # Move data back to original structure
   cd /mnt/md/data
   mv coinbase/* .
   rmdir coinbase
   ```

3. **Start Old Logger**
   ```bash
   sudo systemctl start coinbase-logger
   ```

## Post-Migration Validation

1. **Check Logger Health**
   ```bash
   curl "http://localhost:8080/health"
   ```

2. **Verify Data Collection**
   ```bash
   # Check for new files being created
   ls -la /mnt/md/data/coinbase/BTC-USD/MD/
   ls -la /mnt/md/data/binance/BTC-USDT/MD/
   ```

3. **Monitor Logs**
   ```bash
   journalctl -u logger -f
   ```

## Benefits of Migration

1. **Multi-Exchange Support**: Collect data from multiple exchanges simultaneously
2. **Unified Data Format**: Consistent data structure across all exchanges
3. **Symbol Mapping**: Automatic symbol normalization (e.g., BTC-USD ↔ BTCUSDT)
4. **Better Architecture**: Modular design for easy addition of new exchanges
5. **Improved Testing**: Comprehensive test coverage for all components
6. **Configuration Management**: Centralized configuration with hot-reload support

## Future Considerations

1. **Additional Exchanges**: The architecture supports easy addition of new exchanges (Kraken, Bybit, etc.)
2. **Data Synchronization**: Consider implementing cross-exchange time synchronization
3. **Monitoring**: Set up proper monitoring for each exchange connection
4. **Alerting**: Configure alerts for connection failures or data gaps

## Support

For issues or questions during migration:
1. Check logger logs: `journalctl -u logger -n 100`
2. Check server logs: `journalctl -u gpu-charts-server -n 100`
3. Verify file permissions: All data files should be readable by the server process
4. Ensure adequate disk space for new exchange data