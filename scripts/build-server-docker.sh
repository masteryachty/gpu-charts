#!/bin/bash
# Build script for server Docker image
# Optimized for Cloudflare Tunnel (HTTP/2 cleartext mode)

set -e

echo "🚇 Building server for Cloudflare Tunnel mode..."
echo "📝 Configuration: HTTP/1.1 on port 8443"

echo ""
echo "🔨 Building Docker image..."
docker build -f server/Dockerfile -t gpu-charts-server:latest .

echo "✅ Docker image built successfully: gpu-charts-server:latest"
echo ""
echo "🚀 To deploy:"
echo "   npm run docker:deploy:server"
echo ""
echo "📖 See server/CLOUDFLARE_TUNNEL_SETUP.md for configuration details"