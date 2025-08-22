#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================="
echo "GPU Charts Monitoring Debug Script"
echo "========================================="

# Configuration
LOGGER_URL="${LOGGER_URL:-http://api.rednax.io:9090}"
SERVER_URL="${SERVER_URL:-http://api.rednax.io:9091}"
PROMETHEUS_URL="${PROMETHEUS_URL:-http://prometheus.rednax.io:9090}"

echo -e "\nConfiguration:"
echo "  Logger URL: $LOGGER_URL"
echo "  Server URL: $SERVER_URL"
echo "  Prometheus URL: $PROMETHEUS_URL"

echo -e "\n${YELLOW}=== Stage 1: Service Metrics Endpoints ===${NC}"

# Test logger metrics
echo -n "Testing Logger metrics endpoint: "
LOGGER_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 5 $LOGGER_URL/metrics)
if [ "$LOGGER_STATUS" = "200" ]; then
    echo -e "${GREEN}✓ OK (HTTP $LOGGER_STATUS)${NC}"
    LOGGER_METRICS=$(curl -s $LOGGER_URL/metrics | grep "^gpu_charts" | wc -l)
    echo "  Found $LOGGER_METRICS gpu_charts metrics"
else
    echo -e "${RED}✗ FAILED (HTTP $LOGGER_STATUS)${NC}"
fi

# Test server metrics
echo -n "Testing Server metrics endpoint: "
SERVER_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 5 $SERVER_URL/metrics)
if [ "$SERVER_STATUS" = "200" ]; then
    echo -e "${GREEN}✓ OK (HTTP $SERVER_STATUS)${NC}"
    SERVER_METRICS=$(curl -s $SERVER_URL/metrics | grep "^gpu_charts" | wc -l)
    echo "  Found $SERVER_METRICS gpu_charts metrics"
else
    echo -e "${RED}✗ FAILED (HTTP $SERVER_STATUS)${NC}"
fi

echo -e "\n${YELLOW}=== Stage 2: Sample Metrics ===${NC}"

if [ "$LOGGER_STATUS" = "200" ]; then
    echo "Logger metrics (first 3):"
    curl -s $LOGGER_URL/metrics | grep "^gpu_charts" | head -3 | sed 's/^/  /'
fi

if [ "$SERVER_STATUS" = "200" ]; then
    echo -e "\nServer metrics (first 3):"
    curl -s $SERVER_URL/metrics | grep "^gpu_charts" | head -3 | sed 's/^/  /'
fi

echo -e "\n${YELLOW}=== Stage 3: Prometheus Connection ===${NC}"

# Check if Prometheus is accessible
echo -n "Testing Prometheus endpoint: "
PROM_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 5 $PROMETHEUS_URL/api/v1/targets)
if [ "$PROM_STATUS" = "200" ]; then
    echo -e "${GREEN}✓ OK${NC}"
else
    echo -e "${RED}✗ FAILED (HTTP $PROM_STATUS)${NC}"
fi

# Check target status in Prometheus
if [ "$PROM_STATUS" = "200" ]; then
    echo -e "\nPrometheus Target Status:"
    
    # Check logger target
    LOGGER_TARGET=$(curl -s $PROMETHEUS_URL/api/v1/targets | jq -r '.data.activeTargets[] | select(.labels.job=="gpu-charts-logger") | .health' 2>/dev/null)
    if [ "$LOGGER_TARGET" = "up" ]; then
        echo -e "  gpu-charts-logger: ${GREEN}✓ UP${NC}"
    elif [ -n "$LOGGER_TARGET" ]; then
        echo -e "  gpu-charts-logger: ${RED}✗ DOWN${NC}"
        ERROR=$(curl -s $PROMETHEUS_URL/api/v1/targets | jq -r '.data.activeTargets[] | select(.labels.job=="gpu-charts-logger") | .lastError' 2>/dev/null)
        [ -n "$ERROR" ] && [ "$ERROR" != "null" ] && echo "    Error: $ERROR"
    else
        echo -e "  gpu-charts-logger: ${YELLOW}⚠ NOT CONFIGURED${NC}"
    fi
    
    # Check server target
    SERVER_TARGET=$(curl -s $PROMETHEUS_URL/api/v1/targets | jq -r '.data.activeTargets[] | select(.labels.job=="gpu-charts-server") | .health' 2>/dev/null)
    if [ "$SERVER_TARGET" = "up" ]; then
        echo -e "  gpu-charts-server: ${GREEN}✓ UP${NC}"
    elif [ -n "$SERVER_TARGET" ]; then
        echo -e "  gpu-charts-server: ${RED}✗ DOWN${NC}"
        ERROR=$(curl -s $PROMETHEUS_URL/api/v1/targets | jq -r '.data.activeTargets[] | select(.labels.job=="gpu-charts-server") | .lastError' 2>/dev/null)
        [ -n "$ERROR" ] && [ "$ERROR" != "null" ] && echo "    Error: $ERROR"
    else
        echo -e "  gpu-charts-server: ${YELLOW}⚠ NOT CONFIGURED${NC}"
    fi
fi

echo -e "\n${YELLOW}=== Stage 4: Prometheus Data ===${NC}"

if [ "$PROM_STATUS" = "200" ]; then
    # Query for metrics
    echo "Querying Prometheus for gpu_charts metrics..."
    
    METRICS_COUNT=$(curl -s "$PROMETHEUS_URL/api/v1/label/__name__/values" | jq -r '.data[]' | grep "^gpu_charts" | wc -l)
    echo "  Found $METRICS_COUNT gpu_charts metric types in Prometheus"
    
    if [ "$METRICS_COUNT" -gt 0 ]; then
        echo -e "\n  Sample metrics in Prometheus:"
        curl -s "$PROMETHEUS_URL/api/v1/label/__name__/values" | jq -r '.data[]' | grep "^gpu_charts" | head -5 | sed 's/^/    - /'
    fi
fi

echo -e "\n${YELLOW}=== Diagnostic Summary ===${NC}"

# Summarize issues
ISSUES=0

if [ "$LOGGER_STATUS" != "200" ]; then
    echo -e "${RED}✗ Logger metrics endpoint not accessible${NC}"
    echo "  Fix: Check if logger is running and port 9090 is exposed"
    ISSUES=$((ISSUES + 1))
fi

if [ "$SERVER_STATUS" != "200" ]; then
    echo -e "${RED}✗ Server metrics endpoint not accessible${NC}"
    echo "  Fix: Check if server is running and port 9091 is exposed"
    ISSUES=$((ISSUES + 1))
fi

if [ "$LOGGER_TARGET" = "down" ] || [ "$SERVER_TARGET" = "down" ]; then
    echo -e "${RED}✗ Prometheus cannot reach one or more services${NC}"
    echo "  Fix: Check firewall rules and network connectivity"
    ISSUES=$((ISSUES + 1))
fi

if [ "$LOGGER_TARGET" = "" ] || [ "$SERVER_TARGET" = "" ]; then
    echo -e "${YELLOW}⚠ Prometheus not configured for GPU Charts${NC}"
    echo "  Fix: Update prometheus.yml and reload Prometheus"
    ISSUES=$((ISSUES + 1))
fi

if [ "$ISSUES" -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed! Monitoring should be working.${NC}"
    echo "  If dashboards are still empty, check:"
    echo "  1. Grafana data source configuration"
    echo "  2. Dashboard variable settings"
    echo "  3. Time range in Grafana (try 'Last 5 minutes')"
else
    echo -e "\n${RED}Found $ISSUES issue(s) that need fixing.${NC}"
fi

echo -e "\n========================================="