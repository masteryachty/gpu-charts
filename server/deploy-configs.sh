#!/bin/bash
# Deploy symbol configuration files to production server

echo "Deploying symbol configuration files..."

# Create config directory in Docker container if needed
docker exec gpu-charts-server mkdir -p /opt/gpu-charts/configs 2>/dev/null || true

# Copy config files to the container
for config in server/src/symbols/configs/*.json; do
    if [ -f "$config" ]; then
        filename=$(basename "$config")
        echo "Copying $filename..."
        docker cp "$config" gpu-charts-server:/opt/gpu-charts/configs/
    fi
done

echo "Configuration files deployed successfully!"
echo ""
echo "Testing symbol search endpoint..."
curl -s "https://api.rednax.io/api/symbol-search?q=btc" | python3 -m json.tool | head -20 || echo "Test failed"