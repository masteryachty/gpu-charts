#!/bin/bash

# Generate SSL certificate for api.rednax.io
# This creates a self-signed certificate that includes the domain name

DOMAIN="${1:-api.rednax.io}"
CERT_DIR="${2:-./certs}"

echo "Generating SSL certificate for domain: $DOMAIN"

# Create certificate directory
mkdir -p "$CERT_DIR"

# Generate private key
openssl genrsa -out "$CERT_DIR/$DOMAIN.key" 4096

# Create certificate configuration
cat > "$CERT_DIR/cert.conf" <<EOF
[req]
default_bits = 4096
prompt = no
default_md = sha256
distinguished_name = dn
req_extensions = v3_req

[dn]
C=US
ST=California
L=San Francisco
O=Rednax
OU=IT Department
CN=$DOMAIN

[v3_req]
basicConstraints = CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = $DOMAIN
DNS.2 = localhost
DNS.3 = *.rednax.io
IP.1 = 127.0.0.1
IP.2 = ::1
IP.3 = 192.168.1.91
EOF

# Generate self-signed certificate directly (skip CSR step)
openssl req -new -x509 -key "$CERT_DIR/$DOMAIN.key" \
    -out "$CERT_DIR/$DOMAIN.crt" \
    -days 365 \
    -config "$CERT_DIR/cert.conf" \
    -extensions v3_req

# Create combined PEM file
cat "$CERT_DIR/$DOMAIN.crt" "$CERT_DIR/$DOMAIN.key" > "$CERT_DIR/$DOMAIN.pem"

# Set appropriate permissions
chmod 600 "$CERT_DIR/$DOMAIN.key"
chmod 644 "$CERT_DIR/$DOMAIN.crt"
chmod 600 "$CERT_DIR/$DOMAIN.pem"

# Clean up
rm "$CERT_DIR/cert.conf"

echo "Certificate generated successfully!"
echo "  Certificate: $CERT_DIR/$DOMAIN.crt"
echo "  Private Key: $CERT_DIR/$DOMAIN.key"
echo "  Combined PEM: $CERT_DIR/$DOMAIN.pem"

# Verify the certificate
echo ""
echo "Certificate details:"
openssl x509 -in "$CERT_DIR/$DOMAIN.crt" -text -noout | grep -E "Subject:|DNS:|IP:"