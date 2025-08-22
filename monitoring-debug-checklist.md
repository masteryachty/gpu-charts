# Monitoring Debug Checklist

## Stage 1: Verify Services are Exposing Metrics

### 1.1 Check Logger Metrics Endpoint
```bash
# Test locally if services are running locally
curl -s http://localhost:9090/metrics | head -20

# Test on production server
curl -s http://api.rednax.io:9090/metrics | head -20
```

**Expected**: Should see Prometheus metrics in text format like:
```
# HELP gpu_charts_exchange_connection_status Connection status for each exchange
# TYPE gpu_charts_exchange_connection_status gauge
gpu_charts_exchange_connection_status{exchange="coinbase"} 1
```

### 1.2 Check Server Metrics Endpoint
```bash
# Test locally
curl -s http://localhost:9091/metrics | head -20

# Test on production
curl -s http://api.rednax.io:9091/metrics | head -20
```

**Expected**: Should see server metrics like:
```
# HELP gpu_charts_server_http_requests_total Total HTTP requests
# TYPE gpu_charts_server_http_requests_total counter
```

## Stage 2: Network Connectivity

### 2.1 Check if Prometheus Can Reach Services
From the Prometheus server, test connectivity:
```bash
# Test from Prometheus server
curl -s http://api.rednax.io:9090/metrics | head -5
curl -s http://api.rednax.io:9091/metrics | head -5
```

### 2.2 Check Firewall Rules
```bash
# On api.rednax.io server
sudo ufw status | grep -E "9090|9091"

# Or check iptables
sudo iptables -L -n | grep -E "9090|9091"
```

## Stage 3: Prometheus Configuration

### 3.1 Verify Prometheus Configuration
Check the active Prometheus configuration:
```bash
# On Prometheus server
cat /etc/prometheus/prometheus.yml | grep -A 5 "gpu-charts"
```

Should show:
```yaml
  - job_name: 'gpu-charts-logger'
    static_configs:
      - targets: ['api.rednax.io:9090']
  
  - job_name: 'gpu-charts-server'
    static_configs:
      - targets: ['api.rednax.io:9091']
```

### 3.2 Check Prometheus Targets Status
Open in browser: `http://prometheus.rednax.io:9090/targets`

Look for:
- `gpu-charts-logger` - Should show "UP" (green)
- `gpu-charts-server` - Should show "UP" (green)

If showing "DOWN" (red), check the error message.

## Stage 4: Prometheus Data Collection

### 4.1 Query Prometheus Directly
Go to `http://prometheus.rednax.io:9090/graph` and try these queries:

```promql
# Check if any gpu_charts metrics exist
{__name__=~"gpu_charts.*"}

# Check specific metrics
gpu_charts_exchange_connection_status
gpu_charts_server_http_requests_total
up{job=~"gpu-charts.*"}
```

### 4.2 Check Prometheus Logs
```bash
# On Prometheus server
journalctl -u prometheus -n 50 | grep -E "gpu-charts|error|warn"

# Or if using Docker
docker logs prometheus 2>&1 | grep -E "gpu-charts|error|warn" | tail -20
```

## Stage 5: Grafana Data Source

### 5.1 Test Data Source Connection
In Grafana:
1. Go to Configuration → Data Sources
2. Click on your Prometheus data source
3. Scroll down and click "Save & Test"

### 5.2 Test Query in Grafana
In Grafana:
1. Go to Explore (compass icon)
2. Select Prometheus data source
3. Try a simple query: `up`
4. Try GPU Charts query: `gpu_charts_exchange_connection_status`

## Stage 6: Dashboard Configuration

### 6.1 Check Dashboard Variables
In each dashboard:
1. Go to Dashboard Settings (gear icon)
2. Check Variables section
3. Ensure datasource variable is set correctly

### 6.2 Check Individual Panel Queries
1. Edit a panel (click title → Edit)
2. Check the query
3. Look for any errors in the query editor

## Stage 7: Common Issues & Solutions

### Issue 1: Ports Not Accessible
```bash
# On api.rednax.io, ensure ports are exposed
sudo ufw allow 9090/tcp
sudo ufw allow 9091/tcp
sudo ufw reload
```

### Issue 2: Docker Network Issues
```bash
# Check if containers are exposing ports
docker ps | grep -E "9090|9091"

# Should show:
# 0.0.0.0:9090->9090/tcp for logger
# 0.0.0.0:9091->9091/tcp for server
```

### Issue 3: Prometheus Not Reloaded
```bash
# Reload Prometheus configuration
curl -X POST http://prometheus.rednax.io:9090/-/reload

# Or restart service
sudo systemctl restart prometheus
```

### Issue 4: Wrong Metrics Names
Check actual metric names:
```bash
# Get all metric names from logger
curl -s http://api.rednax.io:9090/metrics | grep "^gpu_charts" | cut -d'{' -f1 | sort -u

# Get all metric names from server
curl -s http://api.rednax.io:9091/metrics | grep "^gpu_charts" | cut -d'{' -f1 | sort -u
```

## Quick Test Script

```bash
#!/bin/bash

echo "=== Stage 1: Testing Service Endpoints ==="
echo -n "Logger metrics (9090): "
curl -s -o /dev/null -w "%{http_code}" http://api.rednax.io:9090/metrics
echo ""

echo -n "Server metrics (9091): "
curl -s -o /dev/null -w "%{http_code}" http://api.rednax.io:9091/metrics
echo ""

echo -e "\n=== Stage 2: Sample Metrics ==="
echo "Logger metrics sample:"
curl -s http://api.rednax.io:9090/metrics | grep "^gpu_charts" | head -3

echo -e "\nServer metrics sample:"
curl -s http://api.rednax.io:9091/metrics | grep "^gpu_charts" | head -3

echo -e "\n=== Stage 3: Prometheus Targets ==="
curl -s http://prometheus.rednax.io:9090/api/v1/targets | jq '.data.activeTargets[] | select(.labels.job | contains("gpu-charts")) | {job: .labels.job, health: .health, lastError: .lastError}'
```

Save this as `test-monitoring.sh` and run it to quickly check all stages.