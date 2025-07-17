#!/bin/bash
set -e

echo "Starting GPU Charts Server..."
echo "Domain: ${DOMAIN}"
echo "Use Let's Encrypt: ${USE_LETSENCRYPT}"

# If Let's Encrypt is enabled and we don't have certificates yet
if [ "${USE_LETSENCRYPT}" = "true" ]; then
    echo "Let's Encrypt is enabled"
    
    # Check if Let's Encrypt certificates exist
    if [ -f "/etc/letsencrypt/live/${DOMAIN}/fullchain.pem" ] && [ -f "/etc/letsencrypt/live/${DOMAIN}/privkey.pem" ]; then
        echo "Using existing Let's Encrypt certificates"
        export SSL_CERT_PATH="/etc/letsencrypt/live/${DOMAIN}/fullchain.pem"
        export SSL_PRIVATE_FILE="/etc/letsencrypt/live/${DOMAIN}/privkey.pem"
    else
        echo "Let's Encrypt certificates not found. To obtain them:"
        echo "1. Make sure port 80 is accessible from the internet"
        echo "2. Run: certbot certonly --standalone -d ${DOMAIN} --email ${EMAIL} --agree-tos --non-interactive"
        echo "Using built-in certificates for api.rednax.io instead"
    fi
else
    echo "Using built-in certificates for api.rednax.io"
fi

# Start the server
exec /app/server