# OKX Exchange Integration

This module implements the OKX exchange integration for the cryptocurrency data logger.

## Features

- **REST API Integration**: Fetches available trading symbols from OKX's REST API
- **WebSocket Support**: Real-time market data and trade feeds via WebSocket
- **Symbol Normalization**: Converts OKX symbol formats to normalized format (already uses dash-separated format)
- **Automatic Reconnection**: Handles disconnections with exponential backoff
- **Ping/Pong Support**: Maintains connection with 30-second ping intervals
- **Batch Subscriptions**: Efficient subscription to multiple symbols

## API Endpoints

- **REST API**: `https://www.okx.com/api/v5/`
- **WebSocket**: `wss://ws.okx.com:8443/ws/v5/public`

## Message Format

### Ticker Data
```json
{
  "instId": "BTC-USDT",
  "last": "43508.1",
  "lastSz": "0.00001",
  "askPx": "43508.1",
  "askSz": "0.0001",
  "bidPx": "43508",
  "bidSz": "0.001",
  "open24h": "43000",
  "ts": "1597026383085"
}
```

### Trade Data
```json
{
  "instId": "BTC-USDT",
  "tradeId": "242720720",
  "px": "43508.1",
  "sz": "0.00001",
  "side": "buy",
  "ts": "1597026383085"
}
```

## Configuration

Enable OKX in your configuration file:

```yaml
exchanges:
  okx:
    enabled: true
    ws_endpoint: wss://ws.okx.com:8443/ws/v5/public
    rest_endpoint: https://www.okx.com/api/v5
    max_connections: 10
    symbols_per_connection: 100
    reconnect_delay_secs: 1
    max_reconnect_delay_secs: 60
    ping_interval_secs: 30
    symbols: [BTC-USDT, ETH-USDT]  # Optional: specify symbols
```

## Testing

Run the OKX connection test:
```bash
cargo run -- test okx
```

List available symbols:
```bash
cargo run -- symbols okx
```

Run with only OKX enabled:
```bash
cargo run -- run --exchanges okx
```