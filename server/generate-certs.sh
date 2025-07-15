#!/bin/bash

# Certificate generation script for Docker build
# This script generates self-signed certificates inside the Docker container

set -e

CERT_DIR="/app/certs"
CERT_FILE="$CERT_DIR/localhost.crt"
KEY_FILE="$CERT_DIR/localhost.key"
CERT_VALIDITY_DAYS=365

echo "🔐 Generating SSL certificates for Docker container..."

# Create certificate directory
mkdir -p "$CERT_DIR"

# Generate private key
openssl genrsa -out "$KEY_FILE" 2048

# Generate self-signed certificate with proper extensions
openssl req -new -x509 -key "$KEY_FILE" -out "$CERT_FILE" -days "$CERT_VALIDITY_DAYS" \
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
        echo 'DNS.2 = gpu-charts-server'
        echo 'DNS.3 = *.local'
        echo 'IP.1 = 127.0.0.1'
        echo 'IP.2 = ::1'
        echo 'IP.3 = 0.0.0.0'
    )

# Set proper permissions
chmod 600 "$KEY_FILE"
chmod 644 "$CERT_FILE"

echo "✅ SSL certificates generated successfully"
echo "📍 Certificate: $CERT_FILE"
echo "📍 Private key: $KEY_FILE"

# Display certificate info
echo ""
echo "📋 Certificate Information:"
openssl x509 -in "$CERT_FILE" -noout -subject -issuer -dates