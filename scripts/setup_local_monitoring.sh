#!/bin/bash

# Setup script for local monitoring stack
# This sets up a local Prometheus Push Gateway for testing

set -e

echo "========================================="
echo "GPU Charts Local Monitoring Setup"
echo "========================================="
echo ""
echo "This script will set up a local monitoring stack for testing."
echo "For production, you'll need to configure a push gateway at prometheus.rednax.io"
echo ""

# Check if docker-compose is available
if ! command -v docker-compose &> /dev/null; then
    echo "ERROR: docker-compose is not installed"
    echo "Please install docker-compose first"
    exit 1
fi

echo "Starting local monitoring stack..."
echo ""

# Start the monitoring stack
docker-compose -f docker-compose.monitoring.yml up -d

echo ""
echo "Waiting for services to start..."
sleep 10

# Check services
echo ""
echo "Checking service status:"
echo ""

# Check Push Gateway
echo -n "Push Gateway (localhost:9091)... "
if curl -s -f -o /dev/null "http://localhost:9091/metrics"; then
    echo "✓ Running"
else
    echo "✗ Not responding"
fi

# Check Prometheus
echo -n "Prometheus (localhost:9090)... "
if curl -s -f -o /dev/null "http://localhost:9090/api/v1/query?query=up"; then
    echo "✓ Running"
else
    echo "✗ Not responding"
fi

# Check Grafana
echo -n "Grafana (localhost:3000)... "
if curl -s -f -o /dev/null "http://localhost:3000/api/health"; then
    echo "✓ Running"
else
    echo "✗ Not responding"
fi

# Check Alertmanager
echo -n "Alertmanager (localhost:9093)... "
if curl -s -f -o /dev/null "http://localhost:9093/-/healthy"; then
    echo "✓ Running"
else
    echo "✗ Not responding"
fi

echo ""
echo "========================================="
echo "Local Monitoring Stack Setup Complete!"
echo "========================================="
echo ""
echo "Services are available at:"
echo "  - Push Gateway:  http://localhost:9091"
echo "  - Prometheus:    http://localhost:9090"
echo "  - Grafana:       http://localhost:3000 (admin/admin)"
echo "  - Alertmanager:  http://localhost:9093"
echo ""
echo "To test with local monitoring:"
echo "  export PROMETHEUS_PUSH_GATEWAY_URL=http://localhost:9091"
echo "  ./scripts/test_monitoring.sh"
echo ""
echo "To start services with local monitoring:"
echo "  PROMETHEUS_PUSH_GATEWAY_URL=http://localhost:9091 ./target/release/logger"
echo "  PROMETHEUS_PUSH_GATEWAY_URL=http://localhost:9091 ./target/release/server"
echo ""
echo "To stop the monitoring stack:"
echo "  docker-compose -f docker-compose.monitoring.yml down"
echo ""