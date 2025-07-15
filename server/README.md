# GPU Charts Server

High-performance HTTP/2 TLS server for serving financial time-series data with ultra-low latency.

## Quick Start

```bash
docker run -d \
  --name gpu-charts-server \
  -p 8443:8443 \
  -v /path/to/your/data:/data:ro \
  masteryachty/gpu-charts-server:latest
```

## Features

- **HTTP/2 with TLS** - Modern, secure protocol with multiplexing
- **Memory-mapped I/O** - Zero-copy data access for ultra-low latency
- **Binary data format** - Efficient 4-byte records for time-series data
- **Multi-day queries** - Automatic date range handling
- **CORS enabled** - Ready for web frontend integration
- **Memory locking** - Optional mlock() for consistent latency

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATA_PATH` | Path to data directory inside container | `/data` |
| `PORT` | HTTPS port to listen on | `8443` |
| `SSL_CERT_PATH` | Path to SSL certificate | `/app/certs/localhost.crt` |
| `SSL_KEY_PATH` | Path to SSL private key | `/app/certs/localhost.key` |
| `RUST_LOG` | Log level (error, warn, info, debug, trace) | `info` |

### Volume Mounts

- `/data` - Mount your time-series data directory here (read-only recommended)
- `/app/certs` - Optional: Mount custom SSL certificates

## Data Format

The server expects data organized as:
```
/data/
├── {symbol}/          # e.g., BTC-USD/
│   └── {type}/        # e.g., MD/
│       ├── time.{DD}.{MM}.{YY}.bin
│       ├── best_bid.{DD}.{MM}.{YY}.bin
│       ├── best_ask.{DD}.{MM}.{YY}.bin
│       └── ...
```

Each `.bin` file contains 4-byte little-endian records sorted by timestamp.

## API Endpoints

### GET /api/symbols
Returns available trading symbols:
```json
{
  "symbols": ["BTC-USD", "ETH-USD", "SOL-USD"]
}
```

### GET /api/data
Serves time-series data with query parameters:
- `symbol` - Trading symbol (e.g., "BTC-USD")
- `type` - Data type (e.g., "MD" for Market Data)
- `start` - Start timestamp (Unix epoch)
- `end` - End timestamp (Unix epoch)
- `columns` - Comma-separated columns (time,best_bid,best_ask,price,volume,side)

Example:
```
https://localhost:8443/api/data?symbol=BTC-USD&type=MD&start=1234567890&end=1234567900&columns=time,best_bid
```

## Docker Compose

```yaml
version: '3.8'

services:
  gpu-charts-server:
    image: masteryachty/gpu-charts-server:latest
    container_name: gpu-charts-server
    ports:
      - "8443:8443"
    volumes:
      - /mnt/md/data:/data:ro
      - ./certs:/app/certs:ro  # Optional: custom certificates
    environment:
      - RUST_LOG=info
      - DATA_PATH=/data
    restart: unless-stopped
```

## SSL Certificates

### Built-in Certificates
The Docker image automatically generates self-signed certificates during the build process. These certificates are suitable for development and include:
- CN=localhost with additional SANs for Docker networking
- 365-day validity period
- Proper TLS extensions for server authentication

### Production Certificates
For production, mount your own certificates to override the built-in ones:
```bash
docker run -d \
  -v /etc/letsencrypt/live/yourdomain.com/fullchain.pem:/app/certs/localhost.crt:ro \
  -v /etc/letsencrypt/live/yourdomain.com/privkey.pem:/app/certs/localhost.key:ro \
  masteryachty/gpu-charts-server:latest
```

## Performance Tuning

### System Requirements
- **Memory**: Depends on data size (memory-mapped files use virtual memory)
- **CPU**: Low CPU usage, benefits from fast single-core performance
- **Storage**: SSD/NVMe recommended for best latency

### Optimization Tips
1. Use local NVMe storage for data files
2. Increase system file descriptor limits
3. Consider disabling swap for consistent latency
4. Use dedicated CPU cores for critical workloads

## Security

- Runs as non-root user (uid 1000)
- TLS encryption for all connections
- Read-only data access recommended
- No authentication built-in (add reverse proxy if needed)

## Source Code

GitHub: [https://github.com/yourusername/gpu-charts](https://github.com/yourusername/gpu-charts)

## License

See LICENSE file in the source repository.