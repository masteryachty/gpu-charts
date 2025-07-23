# Coinbase Logger Docker Deployment Guide

This guide explains how to deploy the Coinbase Logger using Docker.

## Prerequisites

- Docker Engine 20.10+ installed
- Docker Compose 2.0+ installed
- At least 10GB free disk space for data storage
- Linux x86_64 host system

## Quick Start

1. **Set up environment (optional):**
   ```bash
   cd coinbase-logger
   cp .env.example .env
   # Edit .env to set your custom data path
   ```

2. **Build the Docker image:**
   ```bash
   docker-compose build
   ```

3. **Start the container:**
   ```bash
   # Using default path (./data in current directory)
   docker-compose up -d
   
   # Or with custom data path
   DATA_PATH=/path/to/your/data docker-compose up -d
   
   # Or using .env file
   docker-compose up -d
   ```

4. **Check logs:**
   ```bash
   docker-compose logs -f coinbase-logger
   ```

## Configuration

### Data Volume

The logger writes data to `/usr/src/app/data` inside the container. This can be configured in several ways:

**Method 1: Environment Variable (Recommended)**
```bash
# Set via command line
DATA_PATH=/your/custom/path docker-compose up -d

# Or export it
export DATA_PATH=/your/custom/path
docker-compose up -d
```

**Method 2: .env File**
```bash
# Copy the example file
cp .env.example .env

# Edit .env and set DATA_PATH
DATA_PATH=/your/custom/path

# Start normally
docker-compose up -d
```

**Method 3: Direct docker-compose.yml Edit**
```yaml
volumes:
  - type: bind
    source: /your/custom/path
    target: /usr/src/app/data
```

The default path is `./data` (relative to docker-compose.yml location) if not specified.

### Resource Limits

The container is configured with:
- CPU limit: 4 cores
- Memory limit: 2GB
- CPU reservation: 2 cores
- Memory reservation: 1GB

Adjust these in `docker-compose.yml` if needed.

## Operations

### Start/Stop

```bash
# Start
docker-compose up -d

# Stop
docker-compose down

# Restart
docker-compose restart
```

### View Logs

```bash
# All logs
docker-compose logs coinbase-logger

# Follow logs
docker-compose logs -f coinbase-logger

# Last 100 lines
docker-compose logs --tail=100 coinbase-logger
```

### Health Check

The container includes a health check that verifies the process is running:
```bash
docker-compose ps
```

### Automatic File Rotation

The coinbase-logger now includes built-in automatic file rotation at midnight:

- **No Restarts Required**: Files automatically rotate without restarting the container
- **Zero Downtime**: WebSocket connections maintained during rotation
- **Automatic Detection**: Checks every 5 seconds for date changes
- **Seamless Operation**: Flushes data, closes old files, creates new files with new date

This eliminates the need for external restart mechanisms like cron jobs or Docker restarts.

## Monitoring

### Check Data Output

```bash
# List symbol directories (replace ./data with your DATA_PATH)
ls -la ./data/

# Check specific symbol data
ls -la ./data/BTC-USD/MD/

# Monitor file growth
du -sh ./data/*
```

### Container Stats

```bash
# Resource usage
docker stats coinbase-logger

# Process inspection
docker exec coinbase-logger ps aux
```

## Troubleshooting

### Container Won't Start

1. Check logs: `docker-compose logs coinbase-logger`
2. Verify data directory permissions: `ls -ld ./data` (or your DATA_PATH)
3. Ensure sufficient disk space: `df -h ./data`

### No Data Being Written

1. Check WebSocket connectivity: `docker exec coinbase-logger ping -c 4 ws-feed.exchange.coinbase.com`
2. Verify container is running: `docker-compose ps`
3. Check for errors in logs: `docker-compose logs --tail=500 coinbase-logger | grep -i error`

### High Memory Usage

The logger maintains 200+ concurrent WebSocket connections. If memory is an issue:
1. Reduce memory limits in `docker-compose.yml`
2. Monitor with: `docker stats coinbase-logger`

## Security Considerations

- The container runs as non-root user (uid 1000)
- Read-only root filesystem with `/tmp` as tmpfs
- No new privileges flag enabled
- Minimal base image (debian:bookworm-slim)

## Building for Production

For production deployments:

1. **Tag your image:**
   ```bash
   docker build -t your-registry/coinbase-logger:v1.0.0 .
   docker push your-registry/coinbase-logger:v1.0.0
   ```

2. **Update docker-compose.yml:**
   ```yaml
   image: your-registry/coinbase-logger:v1.0.0
   ```

3. **Use environment-specific compose files:**
   ```bash
   docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d
   ```

## Backup Considerations

The binary data files are append-only and rotate daily. To backup:
1. Stop writing to avoid partial records: `docker-compose stop`
2. Copy data files: `rsync -av ./data/ /backup/location/` (or your DATA_PATH)
3. Resume logging: `docker-compose start`

Alternatively, backup previous day's files without stopping (files are complete after midnight rotation).