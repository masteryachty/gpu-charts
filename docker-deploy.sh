#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
DOCKER_USERNAME="${DOCKER_USERNAME:-masteryachty}"
VERSION_TAG="${VERSION_TAG:-latest}"

# Help function
show_help() {
    echo -e "${GREEN}GPU Charts Docker Deployment Script${NC}"
    echo ""
    echo "Usage: $0 [COMMAND] [OPTIONS]"
    echo ""
    echo "Commands:"
    echo "  build-all      Build all Docker images"
    echo "  build-server   Build only the server image"
    echo "  build-logger   Build only the coinbase-logger image"
    echo "  push-all       Push all images to Docker Hub"
    echo "  push-server    Push only the server image"
    echo "  push-logger    Push only the coinbase-logger image"
    echo "  up             Start all services with docker-compose"
    echo "  down           Stop all services"
    echo "  logs           Show logs for all services"
    echo "  logs-server    Show logs for server only"
    echo "  logs-logger    Show logs for coinbase-logger only"
    echo ""
    echo "Options:"
    echo "  DOCKER_USERNAME  Docker Hub username (default: masteryachty)"
    echo "  VERSION_TAG      Version tag (default: latest)"
    echo ""
    echo "Examples:"
    echo "  $0 build-all"
    echo "  $0 push-all"
    echo "  VERSION_TAG=v1.0.0 $0 build-server"
    echo "  $0 up"
}

# Build functions
build_server() {
    echo -e "${BLUE}Building GPU Charts Server...${NC}"
    docker build -f server/Dockerfile -t gpu-charts-server:${VERSION_TAG} .
    docker tag gpu-charts-server:${VERSION_TAG} ${DOCKER_USERNAME}/gpu-charts-server:${VERSION_TAG}
    if [ "${VERSION_TAG}" != "latest" ]; then
        docker tag gpu-charts-server:${VERSION_TAG} ${DOCKER_USERNAME}/gpu-charts-server:latest
    fi
    echo -e "${GREEN}✓ Server built successfully${NC}"
}

build_logger() {
    echo -e "${BLUE}Building Coinbase Logger...${NC}"
    docker build -f coinbase-logger/Dockerfile -t coinbase-logger:${VERSION_TAG} .
    docker tag coinbase-logger:${VERSION_TAG} ${DOCKER_USERNAME}/coinbase-logger:${VERSION_TAG}
    if [ "${VERSION_TAG}" != "latest" ]; then
        docker tag coinbase-logger:${VERSION_TAG} ${DOCKER_USERNAME}/coinbase-logger:latest
    fi
    echo -e "${GREEN}✓ Logger built successfully${NC}"
}

# Push functions
push_server() {
    echo -e "${BLUE}Pushing GPU Charts Server to Docker Hub...${NC}"
    docker push ${DOCKER_USERNAME}/gpu-charts-server:${VERSION_TAG}
    if [ "${VERSION_TAG}" != "latest" ]; then
        docker push ${DOCKER_USERNAME}/gpu-charts-server:latest
    fi
    echo -e "${GREEN}✓ Server pushed successfully${NC}"
}

push_logger() {
    echo -e "${BLUE}Pushing Coinbase Logger to Docker Hub...${NC}"
    docker push ${DOCKER_USERNAME}/coinbase-logger:${VERSION_TAG}
    if [ "${VERSION_TAG}" != "latest" ]; then
        docker push ${DOCKER_USERNAME}/coinbase-logger:latest
    fi
    echo -e "${GREEN}✓ Logger pushed successfully${NC}"
}

# Main command handler
case "$1" in
    build-all)
        build_server
        build_logger
        echo -e "${GREEN}All images built successfully!${NC}"
        ;;
    build-server)
        build_server
        ;;
    build-logger)
        build_logger
        ;;
    push-all)
        push_server
        push_logger
        echo -e "${GREEN}All images pushed successfully!${NC}"
        ;;
    push-server)
        push_server
        ;;
    push-logger)
        push_logger
        ;;
    up)
        echo -e "${BLUE}Starting all services...${NC}"
        docker-compose up -d
        echo -e "${GREEN}Services started. Use '$0 logs' to view logs.${NC}"
        ;;
    down)
        echo -e "${BLUE}Stopping all services...${NC}"
        docker-compose down
        echo -e "${GREEN}Services stopped.${NC}"
        ;;
    logs)
        docker-compose logs -f
        ;;
    logs-server)
        docker-compose logs -f gpu-charts-server
        ;;
    logs-logger)
        docker-compose logs -f coinbase-logger
        ;;
    *)
        show_help
        ;;
esac