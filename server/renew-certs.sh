#!/bin/bash

# Certificate renewal script for production deployment
# This script can be run as a cron job or manually

set -e

# Configuration
CONTAINER_NAME="${CONTAINER_NAME:-gpu-charts-server-production}"
DOMAIN="${DOMAIN:-api.rednax.io}"
EMAIL="${EMAIL:-admin@rednax.io}"
SLACK_WEBHOOK="${SLACK_WEBHOOK:-}"
DISCORD_WEBHOOK="${DISCORD_WEBHOOK:-}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[$(date '+%Y-%m-%d %H:%M:%S')] $1${NC}"
}

warn() {
    echo -e "${YELLOW}[$(date '+%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
}

error() {
    echo -e "${RED}[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}"
}

# Send notification to Slack
send_slack_notification() {
    local message="$1"
    local color="${2:-good}"
    
    if [ -n "$SLACK_WEBHOOK" ]; then
        curl -X POST -H 'Content-type: application/json' \
            --data "{\"attachments\":[{\"color\":\"$color\",\"title\":\"GPU Charts Server - Certificate Renewal\",\"text\":\"$message\",\"footer\":\"$(hostname)\",\"ts\":$(date +%s)}]}" \
            "$SLACK_WEBHOOK" 2>/dev/null || warn "Failed to send Slack notification"
    fi
}

# Send notification to Discord
send_discord_notification() {
    local message="$1"
    local color="${2:-3066993}"  # Green color
    
    if [ -n "$DISCORD_WEBHOOK" ]; then
        curl -X POST -H 'Content-type: application/json' \
            --data "{\"embeds\":[{\"color\":$color,\"title\":\"GPU Charts Server - Certificate Renewal\",\"description\":\"$message\",\"footer\":{\"text\":\"$(hostname)\"},\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%S.%3NZ)\"}]}" \
            "$DISCORD_WEBHOOK" 2>/dev/null || warn "Failed to send Discord notification"
    fi
}

# Check if certificate needs renewal
check_certificate_expiry() {
    log "Checking certificate expiry for $DOMAIN"
    
    # Get certificate expiry date
    local cert_info
    cert_info=$(echo | openssl s_client -servername "$DOMAIN" -connect "$DOMAIN:8443" 2>/dev/null | openssl x509 -noout -dates 2>/dev/null)
    
    if [ $? -eq 0 ]; then
        local not_after
        not_after=$(echo "$cert_info" | grep "notAfter" | cut -d= -f2)
        
        # Convert to seconds since epoch
        local expiry_timestamp
        expiry_timestamp=$(date -d "$not_after" +%s)
        local current_timestamp
        current_timestamp=$(date +%s)
        
        # Calculate days until expiry
        local days_until_expiry
        days_until_expiry=$(( (expiry_timestamp - current_timestamp) / 86400 ))
        
        log "Certificate expires in $days_until_expiry days"
        
        # Return 0 if renewal needed (less than 30 days)
        if [ $days_until_expiry -lt 30 ]; then
            log "Certificate renewal needed"
            return 0
        else
            log "Certificate renewal not needed"
            return 1
        fi
    else
        warn "Could not check certificate expiry"
        return 0  # Assume renewal needed if we can't check
    fi
}

# Renew certificate in container
renew_certificate() {
    log "Renewing certificate for $DOMAIN"
    
    # Check if container is running
    if ! docker ps | grep -q "$CONTAINER_NAME"; then
        error "Container $CONTAINER_NAME is not running"
        return 1
    fi
    
    # Execute renewal command in container
    if docker exec "$CONTAINER_NAME" /app/letsencrypt-setup.sh renew; then
        log "Certificate renewed successfully"
        
        # Restart server to reload certificate
        log "Restarting server to reload certificate"
        docker restart "$CONTAINER_NAME"
        
        # Wait for server to start
        log "Waiting for server to start..."
        sleep 30
        
        # Verify server is responding
        if curl -f -s -k "https://$DOMAIN:8443/api/symbols" > /dev/null; then
            log "Server is responding after certificate renewal"
            send_slack_notification "✅ Certificate renewed successfully for $DOMAIN" "good"
            send_discord_notification "✅ Certificate renewed successfully for $DOMAIN" 3066993
            return 0
        else
            error "Server is not responding after certificate renewal"
            send_slack_notification "❌ Server not responding after certificate renewal for $DOMAIN" "danger"
            send_discord_notification "❌ Server not responding after certificate renewal for $DOMAIN" 15158332
            return 1
        fi
    else
        error "Certificate renewal failed"
        send_slack_notification "❌ Certificate renewal failed for $DOMAIN" "danger"
        send_discord_notification "❌ Certificate renewal failed for $DOMAIN" 15158332
        return 1
    fi
}

# Check container health
check_container_health() {
    log "Checking container health"
    
    # Check if container is running
    if ! docker ps | grep -q "$CONTAINER_NAME"; then
        error "Container $CONTAINER_NAME is not running"
        return 1
    fi
    
    # Check health status
    local health_status
    health_status=$(docker inspect --format='{{.State.Health.Status}}' "$CONTAINER_NAME" 2>/dev/null)
    
    if [ "$health_status" = "healthy" ]; then
        log "Container is healthy"
        return 0
    else
        warn "Container health status: $health_status"
        return 1
    fi
}

# Main function
main() {
    log "Starting certificate renewal check for $DOMAIN"
    
    # Check if we should renew
    if check_certificate_expiry; then
        log "Certificate renewal is needed"
        
        # Perform renewal
        if renew_certificate; then
            log "Certificate renewal completed successfully"
            
            # Verify container health after renewal
            if check_container_health; then
                log "Container health check passed"
            else
                warn "Container health check failed after renewal"
            fi
        else
            error "Certificate renewal failed"
            exit 1
        fi
    else
        log "Certificate renewal not needed"
        
        # Still check container health
        if ! check_container_health; then
            warn "Container health check failed"
            send_slack_notification "⚠️ Container health check failed for $DOMAIN" "warning"
            send_discord_notification "⚠️ Container health check failed for $DOMAIN" 16776960
        fi
    fi
    
    log "Certificate renewal check completed"
}

# Handle command line arguments
case "${1:-check}" in
    "check")
        main
        ;;
    "force")
        log "Forcing certificate renewal"
        if renew_certificate; then
            log "Forced certificate renewal completed"
        else
            error "Forced certificate renewal failed"
            exit 1
        fi
        ;;
    "health")
        check_container_health
        ;;
    "expiry")
        check_certificate_expiry
        ;;
    *)
        echo "Usage: $0 {check|force|health|expiry}"
        echo ""
        echo "Commands:"
        echo "  check   - Check if renewal is needed and renew if necessary (default)"
        echo "  force   - Force certificate renewal"
        echo "  health  - Check container health"
        echo "  expiry  - Check certificate expiry"
        echo ""
        echo "Environment variables:"
        echo "  CONTAINER_NAME - Docker container name (default: gpu-charts-server-production)"
        echo "  DOMAIN         - Domain name (default: api.rednax.io)"
        echo "  EMAIL          - Email for Let's Encrypt (default: admin@rednax.io)"
        echo "  SLACK_WEBHOOK  - Slack webhook URL for notifications (optional)"
        echo "  DISCORD_WEBHOOK - Discord webhook URL for notifications (optional)"
        exit 1
        ;;
esac