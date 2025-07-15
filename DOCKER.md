# Docker Deployment Guide

This guide covers deploying the GPU Charts server and Coinbase Logger using Docker.

## Quick Start

### Build and Run Locally

```bash
# Build all images
./docker-deploy.sh build-all

# Start all services
./docker-deploy.sh up

# View logs
./docker-deploy.sh logs
```

### Deploy to Production

1. **Push to Docker Hub:**
```bash
# Login to Docker Hub
docker login

# Build and push all images
VERSION_TAG=v1.0.0 ./docker-deploy.sh build-all
VERSION_TAG=v1.0.0 ./docker-deploy.sh push-all
```

2. **On Production Server:**
```bash
# Download production compose file
wget https://raw.githubusercontent.com/yourusername/gpu-charts/main/docker-compose.prod.yml

# Update SSL certificate paths in docker-compose.prod.yml
# Replace /etc/letsencrypt/live/yourdomain.com with your cert paths

# Start services
docker-compose -f docker-compose.prod.yml up -d
```

## Services

### GPU Charts Server
- **Port:** 8443 (HTTPS)
- **Image:** `masteryachty/gpu-charts-server`
- **Features:**
  - HTTP/2 with TLS
  - Memory-mapped file serving
  - Ultra-low latency data access
  - CORS enabled

### Coinbase Logger
- **Image:** `masteryachty/coinbase-logger`
- **Features:**
  - Real-time WebSocket data collection
  - Binary file output
  - Automatic reconnection
  - Multi-symbol support

## Configuration

### Environment Variables

**Server:**
- `RUST_LOG`: Log level (default: info)
- `SSL_CERT_PATH`: Path to SSL certificate
- `SSL_KEY_PATH`: Path to SSL private key
- `DATA_PATH`: Path to data directory

**Logger:**
- `RUST_LOG`: Log level (default: info)
- `DATA_OUTPUT_PATH`: Where to write data files

### Volumes

**Server:**
- `/mnt/md/data`: Data directory (read-only)
- `/app/certs`: SSL certificates

**Logger:**
- `/app/data` or `/mnt/md/data`: Data output directory

## SSL Certificates

### Development
The server includes self-signed certificates for localhost. To regenerate:
```bash
npm run setup:ssl
```

### Production
Mount your production certificates:
```yaml
volumes:
  - /path/to/cert.pem:/app/certs/localhost.crt:ro
  - /path/to/key.pem:/app/certs/localhost.key:ro
```

## Docker Commands Reference

```bash
# Build individual services
./docker-deploy.sh build-server
./docker-deploy.sh build-logger

# Push individual services
./docker-deploy.sh push-server
./docker-deploy.sh push-logger

# View logs for specific service
./docker-deploy.sh logs-server
./docker-deploy.sh logs-logger

# Stop all services
./docker-deploy.sh down
```

## Troubleshooting

### Certificate Issues
If you get SSL errors:
1. Check certificate file permissions
2. Ensure certificates are mounted correctly
3. Verify certificate paths in environment variables

### Data Access Issues
If the server can't read data:
1. Check data directory permissions
2. Verify volume mounts
3. Ensure data files exist in expected format

### Network Issues
If services can't communicate:
1. Check they're on the same Docker network
2. Verify service names in connection strings
3. Check firewall rules for port 8443

## Health Checks

Both services include health checks:
- **Server:** TCP connection to port 8443
- **Logger:** Process existence check

Monitor health status:
```bash
docker ps
docker inspect <container_name> | grep -A 10 Health
```