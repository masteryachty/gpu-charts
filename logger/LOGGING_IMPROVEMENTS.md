# Logging Improvements

## Issue
The logger was producing excessive log output, particularly during the subscription phase where it logged each individual channel subscription at INFO level. For example:
- OKX: Would log ~1500 individual subscription confirmations (752 symbols × 2 channels)
- Kraken: Would log ~100 individual subscription confirmations
- Bitfinex: Would log ~60 individual subscription confirmations

## Solution
Changed individual subscription confirmations from INFO to DEBUG level across all exchanges:

### Files Modified:
1. **OKX** (`src/exchanges/okx/connection.rs`):
   - Line 89: Changed `info!("Subscribed to channel: {}", value["arg"]);` to `debug!`

2. **Kraken** (`src/exchanges/kraken/connection.rs`):
   - Line 109: Changed `info!("Subscribed to Kraken channel: {}", channel);` to `debug!`

3. **Bitfinex** (`src/exchanges/bitfinex/connection.rs`):
   - Line 139-142: Changed `info!("Subscribed to {} {} with channel ID {}", ...)` to `debug!`

4. **Coinbase** (`src/exchanges/coinbase/connection.rs`):
   - Line 70: Changed `info!("Subscribed to channels: {}", value["channels"]);` to `debug!`

## Result
- Production logs now only show summary messages like "Subscribed to 100 symbols on OKX"
- Detailed subscription confirmations are still available when running with debug logging
- Significantly reduced log volume during startup
- Cleaner, more readable logs in production

## Running with Debug Logging
If you need to see detailed subscription logs for debugging:
```bash
RUST_LOG=debug cargo run -- run
```