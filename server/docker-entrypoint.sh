#!/bin/bash

# Docker entrypoint script for GPU Charts Server with Let's Encrypt support

set -e

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
    exit 1
}

# Environment variables with defaults
DOMAIN="${DOMAIN:-api.rednax.io}"
EMAIL="${EMAIL:-admin@rednax.io}"
CERT_DIR="${CERT_DIR:-/app/certs}"
USE_LETSENCRYPT="${USE_LETSENCRYPT:-true}"
LETSENCRYPT_STAGING="${LETSENCRYPT_STAGING:-false}"
AUTO_RENEW="${AUTO_RENEW:-true}"

log "Starting GPU Charts Server with Let's Encrypt support"
log "Domain: $DOMAIN"
log "Email: $EMAIL"
log "Use Let's Encrypt: $USE_LETSENCRYPT"
log "Staging: $LETSENCRYPT_STAGING"

# Function to start simple HTTP server for Let's Encrypt challenges
start_http_server() {
    log "Starting HTTP server for Let's Encrypt challenges on port 80"
    
    # Create a simple Python HTTP server in the background
    cd /app/webroot
    python3 -m http.server 80 &
    HTTP_SERVER_PID=$!
    
    # Wait a moment for server to start
    sleep 2
    
    # Check if server started successfully
    if curl -s http://localhost:80 > /dev/null; then
        log "HTTP server started successfully (PID: $HTTP_SERVER_PID)"
        return 0
    else
        warn "HTTP server failed to start"
        return 1
    fi
}

# Function to stop HTTP server
stop_http_server() {
    if [ -n "$HTTP_SERVER_PID" ]; then
        log "Stopping HTTP server (PID: $HTTP_SERVER_PID)"
        kill $HTTP_SERVER_PID 2>/dev/null || true
        wait $HTTP_SERVER_PID 2>/dev/null || true
    fi
}

# Function to setup certificates
setup_certificates() {
    log "Setting up SSL certificates"
    
    if [ "$USE_LETSENCRYPT" = "true" ]; then
        log "Using Let's Encrypt for SSL certificates"
        
        # Check if we already have valid certificates
        if /app/letsencrypt-setup.sh check; then
            log "Valid Let's Encrypt certificates found"
            return 0
        fi
        
        # Need to obtain new certificates
        log "Obtaining new Let's Encrypt certificates"
        
        # Set staging flag if needed
        if [ "$LETSENCRYPT_STAGING" = "true" ]; then
            export STAGING=true
        fi
        
        # Start HTTP server for challenge
        start_http_server
        
        # Obtain certificate
        if /app/letsencrypt-setup.sh obtain; then
            log "Let's Encrypt certificates obtained successfully"
            stop_http_server
            return 0
        else
            error "Failed to obtain Let's Encrypt certificates"
        fi
    else
        log "Using self-signed certificates"
        
        # Check if self-signed certificates exist
        if [ -f "$CERT_DIR/localhost.crt" ] && [ -f "$CERT_DIR/localhost.key" ]; then
            log "Self-signed certificates found"
            return 0
        fi
        
        # Generate self-signed certificates
        log "Generating self-signed certificates"
        if /app/generate-certs.sh; then
            log "Self-signed certificates generated successfully"
            return 0
        else
            error "Failed to generate self-signed certificates"
        fi
    fi
}

# Function to start certificate renewal cron job
start_cron() {
    if [ "$USE_LETSENCRYPT" = "true" ] && [ "$AUTO_RENEW" = "true" ]; then
        log "Starting certificate renewal cron job"
        
        # Start cron service (need to be root for this)
        if [ "$EUID" -eq 0 ]; then
            service cron start
        else
            warn "Cannot start cron service as non-root user"
        fi
    fi
}

# Function to handle graceful shutdown
cleanup() {
    log "Shutting down gracefully..."
    stop_http_server
    
    # Stop server if it's running
    if [ -n "$SERVER_PID" ]; then
        log "Stopping server (PID: $SERVER_PID)"
        kill -TERM "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    
    exit 0
}

# Set up signal handlers
trap cleanup SIGTERM SIGINT

# Main execution
main() {
    log "Initializing GPU Charts Server"
    
    # Ensure directories exist
    mkdir -p "$CERT_DIR" /app/webroot /data
    
    # Setup certificates
    setup_certificates
    
    # Start cron for certificate renewal
    start_cron
    
    # Verify certificates exist
    if [ ! -f "$CERT_DIR/localhost.crt" ] || [ ! -f "$CERT_DIR/localhost.key" ]; then
        error "SSL certificates not found after setup"
    fi
    
    log "SSL certificates verified successfully"
    
    # Start the server
    log "Starting GPU Charts Server"
    
    # Execute the main command
    exec "$@" &
    SERVER_PID=$!
    
    # Wait for server to exit
    wait $SERVER_PID
}

# Run main function
main "$@"