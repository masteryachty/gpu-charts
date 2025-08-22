# GPU Charts Pull-Based Monitoring

This document describes the pull-based monitoring setup where Prometheus scrapes metrics directly from the services.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         GPU Charts Services                          │
│  ┌──────────────────┐              ┌──────────────────────┐        │
│  │   Logger Service │              │   Server API         │        │
│  │   Port: 9090     │              │   Port: 9091         │        │
│  │   /metrics       │              │   /metrics           │        │
│  └────────┬─────────┘              └────────┬─────────────┘        │
│           │                                  │                       │
│           └──────────────┬───────────────────┘                      │
│                          │ HTTP GET /metrics                         │
│                          ▲                                           │
└──────────────────────────────────────────────────────────────────────┘
                           │ Scrape (pull)
                           │
┌─────────────────────────────────────────────────────────────────────┐
│                         Prometheus Server                            │
│                    (prometheus.rednax.io:9090)                       │
│  - Scrapes metrics every 15 seconds                                 │
│  - Stores time-series data                                          │
│  - Evaluates alerting rules                                         │
└──────────┬──────────────────────────────────┬───────────────────────┘
           │ Query                             │ Alerts
           ▼                                   ▼
┌──────────────────────────────┐    ┌────────────────────────────────┐
│         Grafana               │    │     Alertmanager               │
│    (grafana.rednax.io)        │    │  (prometheus.rednax.io:9093)   │
│  - Dashboards                 │    │  - Routes alerts               │
│  - Visualizations             │    │  - Sends notifications         │
└───────────────────────────────┘    └────────────────────────────────┘
```

## Advantages of Pull-Based Monitoring

1. **No Push Gateway Required** - Simpler architecture, one less service to maintain
2. **Service Discovery** - Prometheus can automatically discover new instances
3. **Consistent Scraping** - Prometheus controls the scrape interval
4. **Up Monitoring** - Prometheus knows when a service is down (no scrape response)
5. **Lower Resource Usage** - Services don't need to actively push metrics

## Service Configuration

### Logger Service

The logger exposes metrics on port **9090**:

```bash
# Environment variable to change port
export METRICS_PORT=9090

# Start logger
./target/release/logger

# Metrics available at
http://localhost:9090/metrics
http://localhost:9090/health
```

### Server API

The server exposes metrics on port **9091**:

```bash
# Environment variable to change port
export METRICS_PORT=9091

# Start server
./target/release/ultra_low_latency_server_chunked_parallel

# Metrics available at
http://localhost:9091/metrics
http://localhost:9091/health
```

## Prometheus Configuration

### Basic Configuration (`prometheus.yml`)

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  # Logger metrics
  - job_name: 'gpu-charts-logger'
    static_configs:
      - targets: ['localhost:9090']
        labels:
          instance: 'logger-1'
          environment: 'production'

  # Server metrics
  - job_name: 'gpu-charts-server'
    static_configs:
      - targets: ['localhost:9091']
        labels:
          instance: 'server-1'
          environment: 'production'
```

### Multiple Instances

For multiple instances of each service:

```yaml
scrape_configs:
  - job_name: 'gpu-charts-logger'
    static_configs:
      - targets:
        - 'logger1.example.com:9090'
        - 'logger2.example.com:9090'
        - 'logger3.example.com:9090'
        labels:
          environment: 'production'

  - job_name: 'gpu-charts-server'
    static_configs:
      - targets:
        - 'server1.example.com:9091'
        - 'server2.example.com:9091'
        labels:
          environment: 'production'
```

### Service Discovery

For dynamic environments (Kubernetes, Docker Swarm):

```yaml
scrape_configs:
  # Kubernetes service discovery
  - job_name: 'gpu-charts-logger'
    kubernetes_sd_configs:
      - role: pod
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_label_app]
        regex: gpu-charts-logger
        action: keep
      - source_labels: [__meta_kubernetes_pod_ip]
        target_label: __address__
        replacement: '${1}:9090'

  # Docker service discovery
  - job_name: 'gpu-charts-server'
    docker_sd_configs:
      - host: unix:///var/run/docker.sock
    relabel_configs:
      - source_labels: [__meta_docker_container_label_com_docker_compose_service]
        regex: server
        action: keep
```

## Docker Deployment

### Docker Compose Setup

```yaml
version: '3.8'

services:
  logger:
    image: gpu-charts-logger:latest
    environment:
      - METRICS_PORT=9090
    ports:
      - "9090:9090"  # Metrics endpoint
    networks:
      - monitoring

  server:
    image: gpu-charts-server:latest
    environment:
      - METRICS_PORT=9091
    ports:
      - "9091:9091"  # Metrics endpoint
    networks:
      - monitoring

  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    ports:
      - "9092:9090"
    networks:
      - monitoring

networks:
  monitoring:
```

### Testing with Docker

```bash
# Start the monitoring stack
docker-compose -f docker-compose.monitoring-pull.yml up -d

# Check metrics endpoints
curl http://localhost:9090/metrics  # Logger metrics
curl http://localhost:9091/metrics  # Server metrics

# Check Prometheus targets
curl http://localhost:9092/api/v1/targets

# View in Grafana
open http://localhost:3000
```

## Production Deployment

### 1. Deploy Services with Metrics Endpoints

```bash
# On logger host
METRICS_PORT=9090 ./logger &

# On server host
METRICS_PORT=9091 ./server &
```

### 2. Configure Prometheus at prometheus.rednax.io

Add to Prometheus configuration:

```yaml
scrape_configs:
  - job_name: 'gpu-charts-logger'
    static_configs:
      - targets: ['your-logger-host:9090']

  - job_name: 'gpu-charts-server'
    static_configs:
      - targets: ['your-server-host:9091']
```

### 3. Reload Prometheus

```bash
# Send SIGHUP to reload config
kill -HUP $(pidof prometheus)

# Or via API if enabled
curl -X POST http://prometheus.rednax.io:9090/-/reload
```

### 4. Verify Scraping

```bash
# Check targets status
curl http://prometheus.rednax.io:9090/api/v1/targets | jq '.data.activeTargets[] | {job: .labels.job, health: .health}'

# Query metrics
curl 'http://prometheus.rednax.io:9090/api/v1/query?query=up{job=~"gpu-charts.*"}'
```

## Security Considerations

### 1. Firewall Rules

Only allow Prometheus server to access metrics ports:

```bash
# Allow only Prometheus server IP
iptables -A INPUT -p tcp --dport 9090 -s <prometheus-ip> -j ACCEPT
iptables -A INPUT -p tcp --dport 9090 -j DROP

iptables -A INPUT -p tcp --dport 9091 -s <prometheus-ip> -j ACCEPT
iptables -A INPUT -p tcp --dport 9091 -j DROP
```

### 2. Authentication (Optional)

Add basic auth to metrics endpoints:

```rust
// In metrics_server.rs
use warp::filters::basic::BasicAuth;

let metrics_route = warp::path("metrics")
    .and(warp::get())
    .and(basic_auth("prometheus", "secure-password"))
    .map(|| { /* metrics handler */ });
```

### 3. TLS Encryption (Optional)

For encrypted metrics scraping:

```rust
// Use warp with TLS
warp::serve(routes)
    .tls()
    .cert_path("cert.pem")
    .key_path("key.pem")
    .run(([0, 0, 0, 0], port))
    .await;
```

## Monitoring the Monitors

### Prometheus Self-Monitoring

```yaml
scrape_configs:
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']
```

### Metrics Endpoint Health

Add health checks:

```promql
# Alert if metrics endpoint is down
- alert: MetricsEndpointDown
  expr: up{job=~"gpu-charts.*"} == 0
  for: 5m
  annotations:
    summary: "Metrics endpoint {{ $labels.instance }} is down"
```

## Troubleshooting

### Service Metrics Not Appearing

1. **Check endpoint is accessible:**
```bash
curl http://service-host:9090/metrics
```

2. **Check Prometheus targets:**
```bash
curl http://prometheus.rednax.io:9090/api/v1/targets
```

3. **Check firewall rules:**
```bash
telnet service-host 9090
```

4. **Check service logs:**
```bash
journalctl -u gpu-charts-logger -f
```

### High Memory Usage

Limit metric cardinality:
- Reduce label combinations
- Use recording rules for expensive queries
- Set metric expiration

### Slow Scrapes

Check metric generation performance:
```bash
time curl http://localhost:9090/metrics
```

Optimize if >1 second:
- Cache metric calculations
- Reduce metric count
- Use async metric collection

## Migration from Push to Pull

### 1. Deploy new binaries with metrics endpoints
### 2. Configure Prometheus to scrape endpoints
### 3. Verify metrics are being collected
### 4. Remove push gateway configuration
### 5. Decommission push gateway (if no longer needed)

## Grafana Dashboard Updates

The existing dashboards work with pull-based metrics. Just ensure the datasource is configured correctly:

1. Import dashboards from `/grafana/dashboards/`
2. Select Prometheus datasource
3. Dashboards will automatically use the metrics

## Quick Test Script

```bash
#!/bin/bash
# test-pull-metrics.sh

echo "Testing Pull-Based Metrics"

# Test logger metrics
echo -n "Logger metrics: "
curl -s http://localhost:9090/metrics | grep -c "gpu_charts_exchange"

# Test server metrics
echo -n "Server metrics: "
curl -s http://localhost:9091/metrics | grep -c "gpu_charts_server"

# Test Prometheus scraping
echo -n "Prometheus targets: "
curl -s http://localhost:9092/api/v1/targets | jq '.data.activeTargets | length'

echo "Done!"
```

## Summary

Pull-based monitoring is simpler and more reliable:
- ✅ No push gateway needed
- ✅ Prometheus controls scrape timing
- ✅ Automatic up/down detection
- ✅ Works with existing dashboards
- ✅ Better for long-running services

The services now expose metrics on HTTP endpoints that Prometheus can scrape directly.