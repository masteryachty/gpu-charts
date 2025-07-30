#!/bin/bash

echo "=== Docker Volume Mapping Fix ==="
echo
echo "The issue: Logger expects /mnt/md/data but TrueNAS has /mnt/HDDs/coinbase_logger"
echo

# Current problematic command
echo "CURRENT (incorrect) Docker command:"
echo "docker run -v /mnt/md/data:/mnt/md/data masteryachty/multi-exchange-logger:latest"
echo

# Fixed command
echo "FIXED Docker command:"
echo "docker run -v /mnt/HDDs/coinbase_logger:/mnt/md/data masteryachty/multi-exchange-logger:latest"
echo
echo "This maps the TrueNAS directory to the path the logger expects inside the container."
echo

# Alternative: Update config to use actual path
echo "Alternative - Use config override:"
echo "docker run -v /mnt/HDDs/coinbase_logger:/mnt/HDDs/coinbase_logger \\"
echo "  -v ./config-production.yaml:/app/config.yaml \\"
echo "  masteryachty/multi-exchange-logger:latest"
echo

# Create docker-compose for easier management
cat > docker-compose.yml << 'EOF'
version: '3.8'

services:
  logger:
    image: masteryachty/multi-exchange-logger:latest
    container_name: multi-exchange-logger
    restart: unless-stopped
    volumes:
      # Map TrueNAS path to container's expected path
      - /mnt/HDDs/coinbase_logger:/mnt/md/data
    environment:
      - RUST_LOG=info
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
EOF

echo
echo "Created docker-compose.yml for easier management"
echo
echo "To start the logger with correct volume mapping:"
echo "docker-compose up -d"
echo
echo "To view logs:"
echo "docker-compose logs -f"
echo
echo "To stop:"
echo "docker-compose down"