#!/bin/bash

# Grafana setup script
GRAFANA_URL="${GRAFANA_URL:-http://grafana.rednax.io}"
GRAFANA_USER="${GRAFANA_USER:-admin}"
GRAFANA_PASS="${GRAFANA_PASS:-admin}"

echo "Importing Grafana dashboards to $GRAFANA_URL"

# Function to import a dashboard
import_dashboard() {
    local dashboard_file=$1
    local dashboard_name=$2
    
    echo "Importing $dashboard_name..."
    
    # Read the dashboard JSON
    dashboard_json=$(cat "$dashboard_file")
    
    # Wrap it in the import format
    import_payload=$(cat <<EOF
{
  "dashboard": $dashboard_json,
  "overwrite": true,
  "inputs": [
    {
      "name": "DS_PROMETHEUS",
      "type": "datasource",
      "pluginId": "prometheus",
      "value": "Prometheus"
    }
  ]
}
EOF
)
    
    # Import via API
    curl -X POST \
        -H "Content-Type: application/json" \
        -u "$GRAFANA_USER:$GRAFANA_PASS" \
        -d "$import_payload" \
        "$GRAFANA_URL/api/dashboards/import"
    
    echo ""
}

# Import each dashboard
import_dashboard "grafana/dashboards/exchange-health.json" "Exchange Health"
import_dashboard "grafana/dashboards/server-performance.json" "Server Performance"
import_dashboard "grafana/dashboards/data-quality.json" "Data Quality"

echo "Dashboard import complete!"
echo ""
echo "Access your dashboards at:"
echo "  $GRAFANA_URL/dashboards"