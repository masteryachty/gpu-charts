#!/bin/bash

# SSL Certificate Setup Script for Development

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVER_DIR="$SCRIPT_DIR/../server"
CERT_FILE="$SERVER_DIR/localhost.crt"
KEY_FILE="$SERVER_DIR/localhost.key"

echo "🔐 Setting up SSL certificates for development..."

# Function to check if certificate exists and is valid
check_certificate() {
    if [[ -f "$CERT_FILE" && -f "$KEY_FILE" ]]; then
        # Check if certificate is still valid (not expired)
        if openssl x509 -in "$CERT_FILE" -checkend 86400 -noout >/dev/null 2>&1; then
            echo "✅ Valid SSL certificate found"
            return 0
        else
            echo "⚠️  SSL certificate expired or expiring soon"
            return 1
        fi
    else
        echo "⚠️  SSL certificate files not found"
        return 1
    fi
}

# Function to generate new certificate
generate_certificate() {
    echo "🔨 Generating new SSL certificate..."
    
    # Create server directory if it doesn't exist
    mkdir -p "$SERVER_DIR"
    
    # Generate private key
    openssl genrsa -out "$KEY_FILE" 2048
    
    # Generate certificate signing request and self-signed certificate
    openssl req -new -x509 -key "$KEY_FILE" -out "$CERT_FILE" -days 365 \
        -subj "/CN=localhost" \
        -extensions v3_req \
        -config <(
            echo '[req]'
            echo 'distinguished_name = req'
            echo '[v3_req]'
            echo 'keyUsage = keyEncipherment, dataEncipherment'
            echo 'extendedKeyUsage = serverAuth'
            echo 'subjectAltName = @alt_names'
            echo '[alt_names]'
            echo 'DNS.1 = localhost'
            echo 'DNS.2 = 127.0.0.1'
            echo 'IP.1 = 127.0.0.1'
            echo 'IP.2 = ::1'
        )
    
    # Set proper permissions
    chmod 600 "$KEY_FILE"
    chmod 644 "$CERT_FILE"
    
    echo "✅ SSL certificate generated successfully"
    echo "📍 Certificate: $CERT_FILE"
    echo "📍 Private key: $KEY_FILE"
}

# Main logic
if check_certificate; then
    echo "🎉 SSL setup complete - certificates are ready"
else
    generate_certificate
    echo "🎉 SSL setup complete - new certificates generated"
fi

# Display certificate info
echo ""
echo "📋 Certificate Information:"
openssl x509 -in "$CERT_FILE" -noout -subject -issuer -dates