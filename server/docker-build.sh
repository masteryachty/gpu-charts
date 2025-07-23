#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
IMAGE_NAME="gpu-charts-server"
DOCKER_USERNAME="${DOCKER_USERNAME:-masteryachty}"
VERSION_TAG="${VERSION_TAG:-latest}"

echo -e "${GREEN}Building GPU Charts Server Docker image...${NC}"

# Navigate to project root
cd "$(dirname "$0")/.."

# Build the Docker image
echo -e "${YELLOW}Building ${IMAGE_NAME}:${VERSION_TAG}...${NC}"
docker build -f server/Dockerfile -t ${IMAGE_NAME}:${VERSION_TAG} .

# Tag for Docker Hub
FULL_IMAGE_NAME="${DOCKER_USERNAME}/${IMAGE_NAME}"
docker tag ${IMAGE_NAME}:${VERSION_TAG} ${FULL_IMAGE_NAME}:${VERSION_TAG}

# Also tag as latest if this is not already latest
if [ "${VERSION_TAG}" != "latest" ]; then
    docker tag ${IMAGE_NAME}:${VERSION_TAG} ${FULL_IMAGE_NAME}:latest
    echo -e "${GREEN}Tagged as ${FULL_IMAGE_NAME}:latest${NC}"
fi

echo -e "${GREEN}Successfully built ${FULL_IMAGE_NAME}:${VERSION_TAG}${NC}"
echo -e "${YELLOW}To push to Docker Hub, run: ./server/docker-push.sh${NC}"