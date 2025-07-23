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
FULL_IMAGE_NAME="${DOCKER_USERNAME}/${IMAGE_NAME}"

echo -e "${GREEN}Pushing GPU Charts Server to Docker Hub...${NC}"

# Check if logged in to Docker Hub
if ! docker info 2>/dev/null | grep -q "Username: ${DOCKER_USERNAME}"; then
    echo -e "${YELLOW}Not logged in to Docker Hub. Logging in...${NC}"
    docker login -u ${DOCKER_USERNAME}
fi

# Push the version tag
echo -e "${YELLOW}Pushing ${FULL_IMAGE_NAME}:${VERSION_TAG}...${NC}"
docker push ${FULL_IMAGE_NAME}:${VERSION_TAG}

# Push latest tag if applicable
if [ "${VERSION_TAG}" != "latest" ]; then
    echo -e "${YELLOW}Pushing ${FULL_IMAGE_NAME}:latest...${NC}"
    docker push ${FULL_IMAGE_NAME}:latest
fi

echo -e "${GREEN}Successfully pushed ${FULL_IMAGE_NAME}:${VERSION_TAG} to Docker Hub${NC}"
echo -e "${GREEN}Image available at: https://hub.docker.com/r/${DOCKER_USERNAME}/${IMAGE_NAME}${NC}"