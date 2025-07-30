# Exchange Logger Error Fixes

## Issues Found in Production Logs

### 1. Bitfinex Subscription Limit Error (code 10305)
**Error**: `Bitfinex error (code 10305): subscribe: limit`

**Cause**: Bitfinex has a limit on the number of subscriptions per WebSocket connection. We were trying to subscribe to too many symbols (30 symbols × 2 channels = 60 subscriptions).

**Fix**: 
- Reduced `symbols_per_connection` from 30 to 15 in:
  - `src/config.rs` (default configuration)
  - `config.yaml` (production configuration)
  - `config-local.yaml` (local testing configuration)
- Increased `max_connections` to 10 to maintain coverage

**Result**: Each connection now subscribes to 15 symbols × 2 channels = 30 subscriptions, which should be within Bitfinex's limits.

### 2. Binance I/O Error (os error 5)
**Error**: `Failed to flush data: Input/output error (os error 5)`

**Cause**: This is typically a transient file system error that can occur due to:
- Disk space issues
- Permission problems
- Network storage connectivity issues
- File system corruption

**Recommendations**:
1. **Check disk space**: Ensure `/mnt/md/data/` has sufficient free space
2. **Check permissions**: Verify the logger user has write permissions
3. **Add retry logic**: Implement retry mechanism for flush operations
4. **Monitor disk health**: Check system logs for disk-related errors

**Proposed Code Fix** (to be implemented):
```rust
// In flush_to_disk() method
let mut retries = 3;
let mut delay = Duration::from_millis(100);

while retries > 0 {
    match self.flush_internal().await {
        Ok(_) => return Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::Other && retries > 1 => {
            warn!("I/O error during flush, retrying in {:?}: {}", delay, e);
            tokio::time::sleep(delay).await;
            delay *= 2; // Exponential backoff
            retries -= 1;
        }
        Err(e) => return Err(e),
    }
}
```

## Summary
- **Bitfinex**: Fixed by reducing subscription limits per connection
- **Binance**: I/O errors are likely transient; need monitoring and potentially retry logic
- **OKX & Kraken**: No errors found in the logs, working correctly