# GPU Charts Monitoring Setup

This document describes the comprehensive monitoring infrastructure for GPU Charts, including metrics collection, visualization with Grafana, and alerting via Prometheus.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         GPU Charts Services                          │
│  ┌──────────────────┐              ┌──────────────────────┐        │
│  │   Logger Service │              │   Server API         │        │
│  │   - Exchange     │              │   - HTTP requests    │        │
│  │     connections  │              │   - Data queries     │        │
│  │   - Message rates│              │   - Cache metrics    │        │
│  │   - Error counts │              │   - Response times   │        │
│  └────────┬─────────┘              └────────┬─────────────┘        │
│           │                                  │                       │
│           └──────────────┬───────────────────┘                      │
│                          │ Push metrics                              │
│                          ▼                                           │
└──────────────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Prometheus Push Gateway                           │
│                   (prometheus.rednax.io:9091)                        │
│  - Receives metrics from services via HTTP POST                     │
│  - Aggregates metrics from multiple instances                       │
│  - Exposes metrics for Prometheus scraping                          │
└─────────────────────┬───────────────────────────────────────────────┘
                      │ Scrape
                      ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Prometheus Server                            │
│                    (prometheus.rednax.io:9090)                       │
│  - Stores time-series data                                          │
│  - Evaluates alerting rules                                         │
│  - Serves queries from Grafana                                      │
└──────────┬──────────────────────────────────┬───────────────────────┘
           │ Query                             │ Alerts
           ▼                                   ▼
┌──────────────────────────────┐    ┌────────────────────────────────┐
│         Grafana               │    │     Alertmanager               │
│    (grafana.rednax.io)        │    │  (prometheus.rednax.io:9093)   │
│  - Exchange Health Dashboard  │    │  - Routes alerts               │
│  - Server Performance         │    │  - Sends notifications         │
│  - Data Quality Monitoring    │    │  - Groups/deduplicates         │
└───────────────────────────────┘    └────────────────────────────────┘
```

## Components

### 1. Metrics Collection

#### Logger Service Metrics (`logger/src/metrics_exporter.rs`)

**Exchange Health Metrics:**
- `gpu_charts_exchange_connection_status` - Connection status (0=disconnected, 1=connected)
- `gpu_charts_exchange_messages_total` - Total messages received by type
- `gpu_charts_exchange_errors_total` - Error count by type
- `gpu_charts_exchange_reconnections_total` - Reconnection attempts
- `gpu_charts_websocket_latency_seconds` - WebSocket message latency histogram
- `gpu_charts_symbols_monitored` - Number of symbols per exchange

**Data Quality Metrics:**
- `gpu_charts_last_update_timestamp` - Unix timestamp of last data update
- `gpu_charts_data_gaps_total` - Detected data gaps
- `gpu_charts_trade_volume_total` - Total trade volume
- `gpu_charts_bid_ask_spread` - Current bid-ask spread
- `gpu_charts_vwap_deviation` - VWAP vs price deviation

#### Server API Metrics (`server/src/metrics.rs`)

**HTTP Metrics:**
- `gpu_charts_server_http_requests_total` - Request count by method/endpoint/status
- `gpu_charts_server_http_request_duration_seconds` - Request latency histogram
- `gpu_charts_server_concurrent_connections` - Active connections

**Performance Metrics:**
- `gpu_charts_server_cache_hits_total` - Cache hit count
- `gpu_charts_server_cache_misses_total` - Cache miss count
- `gpu_charts_server_memory_mapped_files` - Number of memory-mapped files
- `gpu_charts_server_data_bytes_served_total` - Total bytes served
- `gpu_charts_server_data_query_duration_seconds` - Data query latency

### 2. Grafana Dashboards

Three comprehensive dashboards are provided in `/grafana/dashboards/`:

#### Exchange Health Dashboard (`exchange-health.json`)
- Real-time connection status for all exchanges
- Message rates per exchange and type
- Error rate tracking
- Reconnection counters
- WebSocket latency percentiles
- Symbol distribution pie chart

#### Server Performance Dashboard (`server-performance.json`)
- Request rate by endpoint and status
- Response time percentiles (p50, p95, p99)
- Cache hit rate gauge
- Memory-mapped file count
- Data throughput graphs
- Query latency tracking
- Concurrent connection monitoring

#### Data Quality Dashboard (`data-quality.json`)
- Data freshness monitoring
- Gap detection visualization
- Trade volume analysis
- Bid-ask spread tracking
- VWAP deviation charts
- Cross-exchange comparison
- Symbol-specific drill-downs
- Data completeness heatmaps

### 3. Alerting Rules

Prometheus alerting rules are configured in `/prometheus/alerts.yml`:

#### Critical Alerts
- **ExchangeDisconnected**: Exchange offline for >2 minutes
- **ServerHighErrorRate**: >1% 5xx errors
- **DataStale**: No updates for >5 minutes

#### Warning Alerts
- **ExchangeNoData**: No messages for 5 minutes
- **ExchangeHighErrorRate**: >10% error rate
- **ExchangeReconnectionLoop**: >5 reconnections in 10 minutes
- **ServerHighResponseTime**: p99 >1 second
- **DataGaps**: >10 gaps per hour

#### Info Alerts
- **ServerLowCacheHitRate**: <50% cache hits
- **AbnormalSpread**: >5% bid-ask spread
- **MetricsPushFailure**: Metrics not being received

## Configuration

### Environment Variables

#### Logger Service
```bash
# Prometheus Push Gateway configuration
PROMETHEUS_PUSH_GATEWAY_URL=http://prometheus.rednax.io
PROMETHEUS_PUSH_INTERVAL_SECS=10  # Default: 10 seconds

# Optional: Override instance name
PROMETHEUS_INSTANCE_NAME=logger-prod-1
```

#### Server API
```bash
# Prometheus Push Gateway configuration
PROMETHEUS_PUSH_GATEWAY_URL=http://prometheus.rednax.io
PROMETHEUS_PUSH_INTERVAL_SECS=10  # Default: 10 seconds

# Optional: Override instance name
PROMETHEUS_INSTANCE_NAME=server-prod-1
```

### Prometheus Configuration

Add to your `prometheus.yml`:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

# Alerting configuration
alerting:
  alertmanagers:
    - static_configs:
        - targets:
            - localhost:9093

# Load alerting rules
rule_files:
  - "alerts.yml"

# Scrape configuration
scrape_configs:
  - job_name: 'pushgateway'
    honor_labels: true
    static_configs:
      - targets: ['localhost:9091']
```

### Alertmanager Configuration

Example `alertmanager.yml`:

```yaml
global:
  resolve_timeout: 5m

route:
  group_by: ['alertname', 'cluster', 'service']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h
  receiver: 'default'
  
  routes:
    - match:
        severity: critical
      receiver: 'critical'
      continue: true
    - match:
        severity: warning
      receiver: 'warning'

receivers:
  - name: 'default'
    # Configure your notification channels here
    
  - name: 'critical'
    email_configs:
      - to: 'oncall@example.com'
        from: 'alerts@example.com'
        headers:
          Subject: 'CRITICAL: GPU Charts Alert'
    
  - name: 'warning'
    slack_configs:
      - api_url: 'YOUR_SLACK_WEBHOOK_URL'
        channel: '#gpu-charts-alerts'
```

## Deployment

### 1. Deploy Prometheus Push Gateway

```bash
docker run -d \
  --name prometheus-pushgateway \
  -p 9091:9091 \
  prom/pushgateway
```

### 2. Deploy Prometheus Server

```bash
# Create config directory
mkdir -p /etc/prometheus

# Copy configuration files
cp prometheus/alerts.yml /etc/prometheus/
cp prometheus.yml /etc/prometheus/

# Run Prometheus
docker run -d \
  --name prometheus \
  -p 9090:9090 \
  -v /etc/prometheus:/etc/prometheus \
  prom/prometheus
```

### 3. Deploy Alertmanager

```bash
docker run -d \
  --name alertmanager \
  -p 9093:9093 \
  -v /etc/alertmanager:/etc/alertmanager \
  prom/alertmanager
```

### 4. Import Grafana Dashboards

1. Log into Grafana at https://grafana.rednax.io
2. Navigate to Dashboards → Import
3. Upload each JSON file from `/grafana/dashboards/`:
   - `exchange-health.json`
   - `server-performance.json`
   - `data-quality.json`
4. Select your Prometheus datasource
5. Click Import

### 5. Start Services with Monitoring

```bash
# Start logger with monitoring
PROMETHEUS_PUSH_GATEWAY_URL=http://prometheus.rednax.io \
./target/release/logger

# Start server with monitoring
PROMETHEUS_PUSH_GATEWAY_URL=http://prometheus.rednax.io \
./target/release/ultra_low_latency_server_chunked_parallel
```

## Testing

### Verify Metrics are Being Pushed

```bash
# Check push gateway metrics
curl http://prometheus.rednax.io/metrics | grep gpu_charts

# Query Prometheus directly
curl 'http://prometheus.rednax.io:9090/api/v1/query?query=gpu_charts_exchange_connection_status'
```

### Test Alerting

```bash
# Simulate exchange disconnection
# Stop the logger service and wait 2 minutes

# Check alert status in Prometheus
curl 'http://prometheus.rednax.io:9090/api/v1/alerts'

# Check Alertmanager
curl http://prometheus.rednax.io:9093/api/v1/alerts
```

### Verify Dashboards

1. Open Grafana dashboards
2. Check that data is flowing
3. Verify all panels are rendering
4. Test time range selection
5. Validate drill-down functionality

## Troubleshooting

### No Metrics in Prometheus

1. Check push gateway is accessible:
   ```bash
   curl http://prometheus.rednax.io/metrics
   ```

2. Verify services are configured correctly:
   ```bash
   grep PROMETHEUS /proc/$(pgrep logger)/environ
   ```

3. Check service logs for push errors:
   ```bash
   journalctl -u gpu-charts-logger -f | grep metrics
   ```

### Dashboards Show "No Data"

1. Verify Prometheus datasource in Grafana
2. Check metric names match:
   ```bash
   curl 'http://prometheus.rednax.io:9090/api/v1/label/__name__/values' | grep gpu_charts
   ```
3. Ensure time range includes recent data

### Alerts Not Firing

1. Check alerting rules are loaded:
   ```bash
   curl http://prometheus.rednax.io:9090/api/v1/rules
   ```

2. Verify alert conditions:
   ```bash
   curl 'http://prometheus.rednax.io:9090/api/v1/query?query=gpu_charts_exchange_connection_status==0'
   ```

3. Check Alertmanager configuration:
   ```bash
   curl http://prometheus.rednax.io:9093/api/v1/status
   ```

## Performance Considerations

### Metric Cardinality
- Avoid high-cardinality labels (e.g., user IDs)
- Use bounded label values (exchanges, symbols)
- Monitor total series count

### Push Frequency
- Default 10-second interval is suitable for most cases
- Increase for high-volume deployments
- Consider batching for multiple instances

### Storage Requirements
- Prometheus: ~2GB per million samples
- 15-second scrape interval = ~5.7M samples/day
- Plan for 15-30 days retention

### Dashboard Optimization
- Use appropriate time ranges
- Limit concurrent queries
- Cache frequently accessed panels
- Use recording rules for complex queries

## Security

### Network Security
- Use TLS for push gateway connections
- Implement authentication for Prometheus/Grafana
- Restrict access to metrics endpoints

### Data Privacy
- Avoid sensitive data in metric labels
- Implement RBAC in Grafana
- Audit metric access

### Secrets Management
- Store webhook URLs securely
- Rotate API keys regularly
- Use environment variables for credentials

## Maintenance

### Regular Tasks
- Review and tune alerting thresholds weekly
- Update dashboards based on feedback
- Archive old metrics data monthly
- Test alert routing quarterly

### Monitoring the Monitoring
- Set up meta-monitoring for Prometheus
- Alert on monitoring system failures
- Regular backup of configurations
- Document any custom queries or rules

## Support

For issues or questions:
1. Check service logs
2. Review this documentation
3. Consult Prometheus/Grafana documentation
4. Open an issue on GitHub

## Appendix

### Useful PromQL Queries

```promql
# Exchange uptime percentage (last 24h)
avg_over_time(gpu_charts_exchange_connection_status[24h]) * 100

# Top 5 slowest API endpoints
topk(5, histogram_quantile(0.95, rate(gpu_charts_server_http_request_duration_seconds_bucket[5m])))

# Data completeness score
(1 - (rate(gpu_charts_data_gaps_total[1h]) / 60)) * 100

# Message processing rate
sum(rate(gpu_charts_exchange_messages_total[1m])) by (exchange)

# Cache effectiveness
sum(rate(gpu_charts_server_cache_hits_total[5m])) / 
(sum(rate(gpu_charts_server_cache_hits_total[5m])) + sum(rate(gpu_charts_server_cache_misses_total[5m])))
```

### Grafana Panel JSON Templates

See `/grafana/dashboards/` for complete examples.

### Metric Naming Conventions

- Prefix: `gpu_charts_`
- Component: `exchange_`, `server_`, `data_`
- Metric type suffix: `_total`, `_seconds`, `_bytes`
- Use underscores, not hyphens
- Be descriptive but concise