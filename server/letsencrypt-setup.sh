#!/bin/bash

# Let's Encrypt Certificate Setup Script for Production
# This script handles both HTTP-01 and DNS-01 challenge methods

set -e

# Configuration
DOMAIN="${DOMAIN:-api.rednax.io}"
EMAIL="${EMAIL:-admin@rednax.io}"
CERT_DIR="${CERT_DIR:-/app/certs}"
WEBROOT_DIR="${WEBROOT_DIR:-/app/webroot}"
STAGING="${STAGING:-false}"

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

# Install certbot if not present
install_certbot() {
    log "Installing certbot..."
    
    if command -v apt-get &> /dev/null; then
        apt-get update
        apt-get install -y certbot python3-certbot-nginx
    elif command -v yum &> /dev/null; then
        yum install -y certbot python3-certbot-nginx
    elif command -v apk &> /dev/null; then
        apk add --no-cache certbot certbot-nginx
    else
        error "Unsupported package manager. Please install certbot manually."
    fi
    
    log "Certbot installed successfully"
}

# Check if certbot is installed
check_certbot() {
    if ! command -v certbot &> /dev/null; then
        log "Certbot not found. Installing..."
        install_certbot
    else
        log "Certbot is already installed"
    fi
}

# Create webroot directory for HTTP-01 challenge
setup_webroot() {
    log "Setting up webroot directory: $WEBROOT_DIR"
    mkdir -p "$WEBROOT_DIR"
    mkdir -p "$CERT_DIR"
    
    # Create a simple index.html for verification
    cat > "$WEBROOT_DIR/index.html" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>GPU Charts Server</title>
</head>
<body>
    <h1>GPU Charts Server</h1>
    <p>SSL Certificate verification endpoint</p>
</body>
</html>
EOF
    
    log "Webroot directory created at $WEBROOT_DIR"
}

# Function to obtain certificate using HTTP-01 challenge
obtain_certificate_http() {
    log "Obtaining Let's Encrypt certificate for $DOMAIN using HTTP-01 challenge"
    
    local staging_flag=""
    if [ "$STAGING" = "true" ]; then
        staging_flag="--staging"
        warn "Using Let's Encrypt staging environment"
    fi
    
    # Run certbot with webroot plugin
    certbot certonly \
        --webroot \
        --webroot-path="$WEBROOT_DIR" \
        --email "$EMAIL" \
        --agree-tos \
        --non-interactive \
        --expand \
        $staging_flag \
        -d "$DOMAIN" \
        --cert-name "$DOMAIN"
    
    if [ $? -eq 0 ]; then
        log "Certificate obtained successfully"
        copy_certificates
    else
        error "Failed to obtain certificate"
    fi
}

# Function to obtain certificate using DNS-01 challenge (for wildcard certs)
obtain_certificate_dns() {
    log "Obtaining Let's Encrypt certificate for $DOMAIN using DNS-01 challenge"
    
    local staging_flag=""
    if [ "$STAGING" = "true" ]; then
        staging_flag="--staging"
        warn "Using Let's Encrypt staging environment"
    fi
    
    warn "DNS-01 challenge requires manual DNS record creation"
    warn "Please add the TXT record as instructed by certbot"
    
    certbot certonly \
        --manual \
        --preferred-challenges dns \
        --email "$EMAIL" \
        --agree-tos \
        --non-interactive \
        --expand \
        $staging_flag \
        -d "$DOMAIN" \
        --cert-name "$DOMAIN"
    
    if [ $? -eq 0 ]; then
        log "Certificate obtained successfully"
        copy_certificates
    else
        error "Failed to obtain certificate"
    fi
}

# Copy certificates to application directory
copy_certificates() {
    log "Copying certificates to $CERT_DIR"
    
    local cert_path="/etc/letsencrypt/live/$DOMAIN"
    
    if [ -d "$cert_path" ]; then
        # Copy certificate files
        cp "$cert_path/fullchain.pem" "$CERT_DIR/localhost.crt"
        cp "$cert_path/privkey.pem" "$CERT_DIR/localhost.key"
        
        # Set proper permissions
        chmod 644 "$CERT_DIR/localhost.crt"
        chmod 600 "$CERT_DIR/localhost.key"
        
        # Change ownership if running as root
        if [ "$EUID" -eq 0 ] && [ -n "$CERT_USER" ]; then
            chown "$CERT_USER:$CERT_USER" "$CERT_DIR/localhost.crt"
            chown "$CERT_USER:$CERT_USER" "$CERT_DIR/localhost.key"
        fi
        
        log "Certificates copied successfully"
        
        # Verify certificate
        verify_certificate
    else
        error "Certificate directory not found: $cert_path"
    fi
}

# Verify certificate
verify_certificate() {
    log "Verifying certificate..."
    
    if openssl x509 -in "$CERT_DIR/localhost.crt" -noout -text | grep -q "$DOMAIN"; then
        log "Certificate verification successful"
        
        # Show certificate details
        log "Certificate details:"
        openssl x509 -in "$CERT_DIR/localhost.crt" -noout -subject -issuer -dates
        
        # Show SAN details
        log "Subject Alternative Names:"
        openssl x509 -in "$CERT_DIR/localhost.crt" -noout -text | grep -A 1 "Subject Alternative Name" || true
        
    else
        error "Certificate verification failed"
    fi
}

# Renew certificate
renew_certificate() {
    log "Renewing Let's Encrypt certificate"
    
    certbot renew --quiet
    
    if [ $? -eq 0 ]; then
        log "Certificate renewed successfully"
        copy_certificates
        
        # Restart server if running (you can customize this)
        if pgrep -f "gpu-charts-server" > /dev/null; then
            log "Restarting server to reload certificates..."
            pkill -HUP -f "gpu-charts-server" || true
        fi
    else
        warn "Certificate renewal failed or not needed"
    fi
}

# Check if certificate exists and is valid
check_certificate() {
    if [ -f "$CERT_DIR/localhost.crt" ]; then
        # Check if certificate is still valid for at least 30 days
        if openssl x509 -checkend 2592000 -noout -in "$CERT_DIR/localhost.crt" >/dev/null 2>&1; then
            log "Certificate is still valid for at least 30 days"
            return 0
        else
            warn "Certificate expires within 30 days"
            return 1
        fi
    else
        warn "No certificate found"
        return 1
    fi
}

# Main function
main() {
    log "Starting Let's Encrypt certificate setup for $DOMAIN"
    
    # Validate inputs
    if [ -z "$DOMAIN" ] || [ -z "$EMAIL" ]; then
        error "DOMAIN and EMAIL must be set"
    fi
    
    case "${1:-obtain}" in
        "obtain")
            check_certbot
            setup_webroot
            
            # Try HTTP-01 challenge first
            obtain_certificate_http
            ;;
        "obtain-dns")
            check_certbot
            obtain_certificate_dns
            ;;
        "renew")
            renew_certificate
            ;;
        "check")
            check_certificate
            ;;
        "verify")
            verify_certificate
            ;;
        *)
            echo "Usage: $0 {obtain|obtain-dns|renew|check|verify}"
            echo ""
            echo "Environment variables:"
            echo "  DOMAIN      - Domain name (default: api.rednax.io)"
            echo "  EMAIL       - Email for Let's Encrypt (default: admin@rednax.io)"
            echo "  CERT_DIR    - Certificate directory (default: /app/certs)"
            echo "  WEBROOT_DIR - Webroot directory (default: /app/webroot)"
            echo "  STAGING     - Use staging environment (default: false)"
            echo "  CERT_USER   - User for certificate ownership (optional)"
            echo ""
            echo "Examples:"
            echo "  $0 obtain           # Obtain certificate using HTTP-01"
            echo "  $0 obtain-dns       # Obtain certificate using DNS-01"
            echo "  $0 renew            # Renew existing certificate"
            echo "  $0 check            # Check certificate validity"
            echo "  $0 verify           # Verify certificate details"
            exit 1
            ;;
    esac
}

# Run main function
main "$@"