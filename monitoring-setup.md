# GPU Charts Monitoring Setup

## Current Status

The monitoring infrastructure has been fully implemented in the codebase with:
- ✅ Prometheus metrics collection in both logger and server
- ✅ Grafana dashboard JSON files ready for import
- ✅ Alerting rules configured
- ✅ Docker Compose for local testing

## Push Gateway Configuration

**Important:** The push gateway at `prometheus.rednax.io` appears to not be configured yet. You'll need to either:

### Option 1: Set up Push Gateway at prometheus.rednax.io
Deploy a Prometheus Push Gateway at prometheus.rednax.io to receive metrics from the services.

### Option 2: Use Local Monitoring Stack for Testing
```bash
# Start local monitoring stack
./scripts/setup_local_monitoring.sh

# Test with local push gateway
export PROMETHEUS_PUSH_GATEWAY_URL=http://localhost:9091
./scripts/test_monitoring.sh

# Run services with local monitoring
PROMETHEUS_PUSH_GATEWAY_URL=http://localhost:9091 ./target/release/logger
PROMETHEUS_PUSH_GATEWAY_URL=http://localhost:9091 ./target/release/server
```

### Option 3: Direct Prometheus Remote Write (Alternative)
If you have Prometheus configured with remote write API enabled, you could modify the implementation to use remote write instead of push gateway.

## Quick Start for Local Testing

1. **Start the local monitoring stack:**
```bash
docker-compose -f docker-compose.monitoring.yml up -d
```

2. **Run services with local monitoring:**
```bash
# In separate terminals:
PROMETHEUS_PUSH_GATEWAY_URL=http://localhost:9091 cargo run --release --package logger
PROMETHEUS_PUSH_GATEWAY_URL=http://localhost:9091 cargo run --release --package server
```

3. **Access monitoring interfaces:**
- Push Gateway: http://localhost:9091
- Prometheus: http://localhost:9090
- Grafana: http://localhost:3000 (admin/admin)
- Alertmanager: http://localhost:9093

4. **Import Grafana dashboards:**
- Log into Grafana
- Go to Dashboards → Import
- Upload JSON files from `/grafana/dashboards/`

## Production Deployment

Once the push gateway is available at prometheus.rednax.io:

1. **Update environment variables:**
```bash
export PROMETHEUS_PUSH_GATEWAY_URL=http://prometheus.rednax.io  # or the correct URL
```

2. **Run services:**
```bash
./target/release/logger
./target/release/server
```

3. **Import dashboards to production Grafana:**
Upload the JSON files from `/grafana/dashboards/` to grafana.rednax.io

## Files Created

- `/logger/src/metrics_exporter.rs` - Logger metrics implementation
- `/server/src/metrics.rs` - Server metrics implementation
- `/grafana/dashboards/*.json` - Three Grafana dashboards
- `/prometheus/alerts.yml` - Alerting rules
- `/docs/MONITORING.md` - Comprehensive documentation
- `/docker-compose.monitoring.yml` - Local monitoring stack
- `/scripts/test_monitoring.sh` - Testing script
- `/scripts/setup_local_monitoring.sh` - Local setup script

## Next Steps

1. Configure push gateway at prometheus.rednax.io OR
2. Use the local monitoring stack for testing OR
3. Consider alternative monitoring solutions (e.g., direct Prometheus scraping if services are accessible)

The implementation is complete and ready to use once the infrastructure is available.