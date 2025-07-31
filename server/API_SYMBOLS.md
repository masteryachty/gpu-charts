# Symbols API Documentation

## Overview
The `/api/symbols` endpoint provides a comprehensive list of all available trading symbols organized by exchange, including the last update timestamp for each symbol.

## Endpoint
```
GET https://localhost:8443/api/symbols
GET https://localhost:8443/api/symbols?exchange={exchange_name}
```

## Query Parameters
- `exchange` (optional): Filter results to show only symbols from a specific exchange (e.g., "coinbase", "binance", "kraken", "okx", "bitfinex")

## Response Format
The endpoint returns a JSON object with the following structure:

```json
{
  "symbols": ["BTC-USD", "ETH-USD", "SOL-USD", ...],
  "exchanges": {
    "coinbase": [
      {
        "symbol": "BTC-USD",
        "last_update": 1738308123,
        "last_update_date": "2025-01-31 12:35:23 UTC"
      },
      {
        "symbol": "ETH-USD", 
        "last_update": 1738308115,
        "last_update_date": "2025-01-31 12:35:15 UTC"
      }
    ],
    "binance": [
      {
        "symbol": "BTCUSDT",
        "last_update": 1738308120,
        "last_update_date": "2025-01-31 12:35:20 UTC"
      }
    ],
    "kraken": [...],
    "okx": [...],
    "bitfinex": [...]
  }
}
```

## Response Fields

### `symbols` (array)
A deduplicated array of all unique symbol names across all exchanges. This provides a quick overview of all available symbols regardless of exchange.

### `exchanges` (object)
An object where each key is an exchange name, and the value is an array of symbol information objects.

### Symbol Information Object
Each symbol within an exchange contains:
- `symbol`: The raw symbol name as used by that exchange (e.g., "BTC-USD" for Coinbase, "BTCUSDT" for Binance)
- `last_update`: Unix timestamp (seconds since epoch) of the most recent data file modification for this symbol
- `last_update_date`: Human-readable date string in UTC format (e.g., "2025-01-31 12:35:23 UTC")

### Sorting
Symbols within each exchange are sorted by `last_update` in descending order (newest first).

## Implementation Details

### Last Update Calculation
The `last_update` timestamp represents the most recent modification time across all data files for a given symbol. The server:
1. Scans all subdirectories (MD, TRADES, etc.) under each symbol
2. Finds all `.bin` files within those directories
3. Returns the most recent modification timestamp among all files

### Directory Structure Scanned
```
/mnt/md/data/
├── {exchange}/
│   └── {symbol}/
│       ├── MD/
│       │   ├── time.{DD}.{MM}.{YY}.bin
│       │   ├── best_bid.{DD}.{MM}.{YY}.bin
│       │   └── ...
│       └── TRADES/
│           └── ...
```

## Usage Examples

### Basic Request
```bash
curl -k -s "https://localhost:8443/api/symbols" | jq
```

### Filter by Exchange
```bash
# Get only Coinbase symbols
curl -k -s "https://localhost:8443/api/symbols?exchange=coinbase" | jq

# Get only Binance symbols  
curl -k -s "https://localhost:8443/api/symbols?exchange=binance" | jq
```

### Get All Coinbase Symbols
```bash
curl -k -s "https://localhost:8443/api/symbols" | jq '.exchanges.coinbase'
```

### Find Symbols Updated in Last Hour
```bash
curl -k -s "https://localhost:8443/api/symbols" | jq --arg cutoff $(($(date +%s) - 3600)) '
  .exchanges | to_entries[] | {
    exchange: .key,
    recent: [.value[] | select(.last_update > ($cutoff | tonumber))]
  } | select(.recent | length > 0)
'
```

### Count Symbols Per Exchange
```bash
curl -k -s "https://localhost:8443/api/symbols" | jq '
  .exchanges | to_entries | map({exchange: .key, count: (.value | length)})
'
```

## Error Responses

### 500 Internal Server Error
Returned when the server cannot read the data directory:
```json
{
  "error": "Failed to read symbol directory"
}
```

## Performance Notes
- The endpoint scans the filesystem on each request to ensure data freshness
- For large numbers of symbols, the response time may increase
- Consider implementing caching if frequent requests are expected

## CORS Support
The endpoint includes CORS headers to allow access from web applications:
- `Access-Control-Allow-Origin: *`
- Preflight OPTIONS requests are supported