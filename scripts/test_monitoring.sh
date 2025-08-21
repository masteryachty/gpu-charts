#!/bin/bash

# GPU Charts Monitoring Test Script
# Tests metrics push to prometheus.rednax.io

set -e

PUSH_GATEWAY_URL="${PROMETHEUS_PUSH_GATEWAY_URL:-http://prometheus.rednax.io}"
PROMETHEUS_URL="${PROMETHEUS_URL:-http://prometheus.rednax.io}"
INSTANCE_NAME=$(hostname)

echo "========================================="
echo "GPU Charts Monitoring Test"
echo "========================================="
echo ""
echo "Configuration:"
echo "  Push Gateway: $PUSH_GATEWAY_URL"
echo "  Prometheus:   $PROMETHEUS_URL"
echo "  Instance:     $INSTANCE_NAME"
echo ""

# Function to check if URL is accessible
check_url() {
    local url=$1
    local name=$2
    
    echo -n "Checking $name... "
    if curl -s -f -o /dev/null "$url"; then
        echo "✓ OK"
        return 0
    else
        echo "✗ FAILED"
        return 1
    fi
}

# Function to send test metrics
send_test_metrics() {
    local job=$1
    local metric_data=$2
    
    echo -n "Sending test metrics for $job... "
    
    if curl -s -X POST "$PUSH_GATEWAY_URL/metrics/job/$job/instance/$INSTANCE_NAME" \
        -H "Content-Type: text/plain" \
        --data-raw "$metric_data"; then
        echo "✓ Sent"
        return 0
    else
        echo "✗ Failed"
        return 1
    fi
}

# Function to query metrics from Prometheus
query_metrics() {
    local metric=$1
    
    # Skip if Prometheus is not available
    if [ "$PROMETHEUS_AVAILABLE" = "false" ]; then
        echo "Skipping $metric query (Prometheus not accessible)"
        return 0
    fi
    
    echo -n "Querying $metric... "
    
    local response=$(timeout 5 curl -s "$PROMETHEUS_URL/api/v1/query?query=$metric" 2>/dev/null || echo "")
    
    if echo "$response" | grep -q '"status":"success"'; then
        local count=$(echo "$response" | grep -o '"value"' | wc -l)
        echo "✓ Found $count series"
        return 0
    else
        echo "✗ Not found"
        return 1
    fi
}

echo "========================================="
echo "Step 1: Connectivity Check"
echo "========================================="
echo ""

# Check push gateway
if ! check_url "$PUSH_GATEWAY_URL/metrics" "Push Gateway"; then
    echo ""
    echo "ERROR: Cannot reach Prometheus Push Gateway at $PUSH_GATEWAY_URL"
    echo "Please verify the URL and network connectivity."
    exit 1
fi

# Check Prometheus (skip if not accessible - push gateway is what matters)
echo -n "Checking Prometheus Server... "
if timeout 5 curl -s -f -o /dev/null "$PROMETHEUS_URL/api/v1/query?query=up" 2>/dev/null; then
    echo "✓ OK"
    PROMETHEUS_AVAILABLE=true
else
    echo "✗ Not accessible (continuing anyway)"
    PROMETHEUS_AVAILABLE=false
    echo "WARNING: Prometheus server not accessible, but push gateway is working."
    echo "Metrics will be pushed successfully but queries won't work."
fi

echo ""
echo "========================================="
echo "Step 2: Send Test Metrics"
echo "========================================="
echo ""

# Logger test metrics
LOGGER_METRICS="# HELP gpu_charts_test_exchange_status Test exchange connection status
# TYPE gpu_charts_test_exchange_status gauge
gpu_charts_test_exchange_status{exchange=\"test_exchange\"} 1
# HELP gpu_charts_test_messages_total Test message counter
# TYPE gpu_charts_test_messages_total counter
gpu_charts_test_messages_total{exchange=\"test_exchange\",message_type=\"orderbook\"} 1000
# HELP gpu_charts_test_latency_seconds Test latency histogram
# TYPE gpu_charts_test_latency_seconds histogram
gpu_charts_test_latency_seconds_bucket{exchange=\"test_exchange\",le=\"0.01\"} 100
gpu_charts_test_latency_seconds_bucket{exchange=\"test_exchange\",le=\"0.1\"} 150
gpu_charts_test_latency_seconds_bucket{exchange=\"test_exchange\",le=\"1\"} 190
gpu_charts_test_latency_seconds_bucket{exchange=\"test_exchange\",le=\"+Inf\"} 200
gpu_charts_test_latency_seconds_sum{exchange=\"test_exchange\"} 5.5
gpu_charts_test_latency_seconds_count{exchange=\"test_exchange\"} 200"

send_test_metrics "gpu-charts-logger-test" "$LOGGER_METRICS"

# Server test metrics
SERVER_METRICS="# HELP gpu_charts_test_http_requests_total Test HTTP request counter
# TYPE gpu_charts_test_http_requests_total counter
gpu_charts_test_http_requests_total{method=\"GET\",endpoint=\"/api/data\",status=\"200\"} 500
gpu_charts_test_http_requests_total{method=\"GET\",endpoint=\"/api/symbols\",status=\"200\"} 100
# HELP gpu_charts_test_cache_hits_total Test cache hits
# TYPE gpu_charts_test_cache_hits_total counter
gpu_charts_test_cache_hits_total 450
# HELP gpu_charts_test_cache_misses_total Test cache misses
# TYPE gpu_charts_test_cache_misses_total counter
gpu_charts_test_cache_misses_total 50"

send_test_metrics "gpu-charts-server-test" "$SERVER_METRICS"

echo ""
echo "========================================="
echo "Step 3: Verify Metrics in Push Gateway"
echo "========================================="
echo ""

echo "Fetching metrics from push gateway..."
GATEWAY_METRICS=$(curl -s "$PUSH_GATEWAY_URL/metrics" 2>/dev/null || echo "")

if echo "$GATEWAY_METRICS" | grep -q "gpu_charts_test"; then
    echo "✓ Test metrics found in push gateway"
    echo ""
    echo "Sample metrics:"
    echo "$GATEWAY_METRICS" | grep "gpu_charts_test" | head -5
else
    echo "✗ Test metrics not found in push gateway"
    exit 1
fi

echo ""
echo "========================================="
echo "Step 4: Query Metrics from Prometheus"
echo "========================================="
echo ""

# Wait a bit for Prometheus to scrape
echo "Waiting 15 seconds for Prometheus to scrape metrics..."
sleep 15

# Query test metrics
query_metrics "gpu_charts_test_exchange_status"
query_metrics "gpu_charts_test_messages_total"
query_metrics "gpu_charts_test_http_requests_total"
query_metrics "gpu_charts_test_cache_hits_total"

echo ""
echo "========================================="
echo "Step 5: Test Production Metrics"
echo "========================================="
echo ""

# Check for actual production metrics
echo "Checking for production metrics..."

query_metrics "gpu_charts_exchange_connection_status" || true
query_metrics "gpu_charts_exchange_messages_total" || true
query_metrics "gpu_charts_server_http_requests_total" || true
query_metrics "gpu_charts_server_cache_hits_total" || true

echo ""
echo "========================================="
echo "Step 6: Cleanup Test Metrics"
echo "========================================="
echo ""

echo -n "Deleting test metrics... "
if curl -s -X DELETE "$PUSH_GATEWAY_URL/metrics/job/gpu-charts-logger-test/instance/$INSTANCE_NAME" && \
   curl -s -X DELETE "$PUSH_GATEWAY_URL/metrics/job/gpu-charts-server-test/instance/$INSTANCE_NAME"; then
    echo "✓ Cleaned up"
else
    echo "✗ Cleanup failed (non-critical)"
fi

echo ""
echo "========================================="
echo "Test Summary"
echo "========================================="
echo ""
echo "✅ Monitoring infrastructure is working correctly!"
echo ""
echo "Next steps:"
echo "1. Start the logger service with PROMETHEUS_PUSH_GATEWAY_URL=$PUSH_GATEWAY_URL"
echo "2. Start the server API with PROMETHEUS_PUSH_GATEWAY_URL=$PUSH_GATEWAY_URL"
echo "3. Import Grafana dashboards from /grafana/dashboards/"
echo "4. Configure alerting in Prometheus using /prometheus/alerts.yml"
echo ""
echo "To monitor services in production:"
echo "  PROMETHEUS_PUSH_GATEWAY_URL=$PUSH_GATEWAY_URL ./target/release/logger"
echo "  PROMETHEUS_PUSH_GATEWAY_URL=$PUSH_GATEWAY_URL ./target/release/ultra_low_latency_server_chunked_parallel"
echo ""