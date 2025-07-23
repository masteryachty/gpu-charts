#!/bin/bash

echo "Fixing Docker Compose 'ContainerConfig' error..."

# Stop and remove existing containers
echo "Stopping existing containers..."
docker-compose down

# Remove old containers and images
echo "Removing old containers and images..."
docker rm -f coinbase-logger 2>/dev/null || true
docker rmi coinbase-logger:latest 2>/dev/null || true

# Clean up Docker system
echo "Cleaning Docker system..."
docker system prune -f

# Rebuild from scratch
echo "Rebuilding image..."
docker-compose build --no-cache

# Start the service
echo "Starting service..."
docker-compose up -d

echo "Done! Check logs with: docker-compose logs -f"