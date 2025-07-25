#!/bin/bash

# GPU Charts Docker Build and Push Script
# This script builds and pushes Docker images for the GPU Charts project locally

set -e

# Configuration
DOCKER_REGISTRY=${DOCKER_REGISTRY:-"docker.io"}
DOCKER_USERNAME=${DOCKER_USERNAME:-""}
SERVER_IMAGE_NAME=${SERVER_IMAGE_NAME:-"gpu-charts-server"}
LOGGER_IMAGE_NAME=${LOGGER_IMAGE_NAME:-"multi-exchange-logger"}
TAG=${TAG:-"latest"}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Functions
print_usage() {
    echo "Usage: $0 [COMMAND] [OPTIONS]"
    echo ""
    echo "Commands:"
    echo "  build-all       Build all Docker images"
    echo "  build-server    Build GPU Charts server image"
    echo "  build-logger    Build Coinbase logger image"
    echo "  push-all        Push all Docker images"
    echo "  push-server     Push GPU Charts server image"
    echo "  push-logger     Push Coinbase logger image"
    echo "  build-push-all  Build and push all images"
    echo "  login           Login to Docker registry"
    echo "  help            Show this help message"
    echo ""
    echo "Options:"
    echo "  --tag TAG               Docker image tag (default: latest)"
    echo "  --registry REGISTRY     Docker registry (default: docker.io)"
    echo "  --username USERNAME     Docker Hub username"
    echo "  --server-name NAME      Server image name (default: gpu-charts-server)"
    echo "  --logger-name NAME      Logger image name (default: multi-exchange-logger)"
    echo "  --no-cache             Build without Docker cache"
    echo ""
    echo "Environment Variables:"
    echo "  DOCKER_REGISTRY    Docker registry URL"
    echo "  DOCKER_USERNAME    Docker Hub username"
    echo "  SERVER_IMAGE_NAME  Server image name"
    echo "  LOGGER_IMAGE_NAME  Logger image name"
    echo "  TAG               Docker image tag"
    echo ""
    echo "Examples:"
    echo "  $0 build-all --tag v1.0.0"
    echo "  $0 push-all --username myuser --tag latest"
    echo "  $0 build-server --production --no-cache"
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Parse command line arguments
COMMAND=""
NO_CACHE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        build-all|build-server|build-logger|push-all|push-server|push-logger|build-push-all|login|help)
            COMMAND=$1
            shift
            ;;
        --tag)
            TAG="$2"
            shift 2
            ;;
        --registry)
            DOCKER_REGISTRY="$2"
            shift 2
            ;;
        --username)
            DOCKER_USERNAME="$2"
            shift 2
            ;;
        --server-name)
            SERVER_IMAGE_NAME="$2"
            shift 2
            ;;
        --logger-name)
            LOGGER_IMAGE_NAME="$2"
            shift 2
            ;;
        --no-cache)
            NO_CACHE="--no-cache"
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            print_usage
            exit 1
            ;;
    esac
done

# Validate command
if [ -z "$COMMAND" ]; then
    log_error "No command specified"
    print_usage
    exit 1
fi

# Construct image names
if [ -n "$DOCKER_USERNAME" ]; then
    SERVER_IMAGE="${DOCKER_REGISTRY}/${DOCKER_USERNAME}/${SERVER_IMAGE_NAME}:${TAG}"
    LOGGER_IMAGE="${DOCKER_REGISTRY}/${DOCKER_USERNAME}/${LOGGER_IMAGE_NAME}:${TAG}"
else
    SERVER_IMAGE="${SERVER_IMAGE_NAME}:${TAG}"
    LOGGER_IMAGE="${LOGGER_IMAGE_NAME}:${TAG}"
fi

# Docker login function
docker_login() {
    if [ -n "$DOCKER_USERNAME" ]; then
        log_info "Logging into Docker registry: $DOCKER_REGISTRY"
        
        # Check if we're already logged in
        if docker info 2>/dev/null | grep -q "Username: $DOCKER_USERNAME"; then
            log_success "Already logged in as $DOCKER_USERNAME"
            return 0
        fi
        
        # Check if running in TTY for interactive login
        if [ -t 0 ]; then
            docker login "$DOCKER_REGISTRY" -u "$DOCKER_USERNAME"
            if [ $? -eq 0 ]; then
                log_success "Successfully logged into Docker registry"
            else
                log_error "Failed to login to Docker registry"
                exit 1
            fi
        else
            log_warning "Non-interactive terminal detected. Please login to Docker manually:"
            log_warning "  docker login -u $DOCKER_USERNAME"
            log_warning "Or set DOCKER_PASSWORD environment variable and use:"
            log_warning "  echo \$DOCKER_PASSWORD | docker login -u $DOCKER_USERNAME --password-stdin"
            exit 1
        fi
    else
        log_warning "No Docker username specified. Assuming already logged in or using local registry."
    fi
}

# Build functions
build_server() {
    log_info "Building GPU Charts server image..."
    log_info "Image: $SERVER_IMAGE"
    
    # Build from root directory with correct context
    cd "$SCRIPT_DIR"
    
    log_info "Building server image with api.rednax.io certificate"
    docker build $NO_CACHE -f server/Dockerfile -t "$SERVER_IMAGE" .
    
    if [ $? -eq 0 ]; then
        log_success "Server image built successfully: $SERVER_IMAGE"
        docker images | grep "${SERVER_IMAGE_NAME}" | grep "$TAG"
    else
        log_error "Failed to build server image"
        exit 1
    fi
    
    cd "$SCRIPT_DIR"
}

build_logger() {
    log_info "Building Multi-Exchange logger image..."
    log_info "Image: $LOGGER_IMAGE"
    
    # Build from the logger directory (Dockerfile expects this context)
    cd "$SCRIPT_DIR/logger"
    
    docker build $NO_CACHE -f Dockerfile -t "$LOGGER_IMAGE" .
    
    if [ $? -eq 0 ]; then
        log_success "Logger image built successfully: $LOGGER_IMAGE"
        docker images | grep "${LOGGER_IMAGE_NAME}" | grep "$TAG"
    else
        log_error "Failed to build logger image"
        exit 1
    fi
    
    cd "$SCRIPT_DIR"
}

# Push functions
push_server() {
    log_info "Pushing server image: $SERVER_IMAGE"
    
    docker push "$SERVER_IMAGE"
    
    if [ $? -eq 0 ]; then
        log_success "Server image pushed successfully"
    else
        log_error "Failed to push server image"
        exit 1
    fi
}

push_logger() {
    log_info "Pushing logger image: $LOGGER_IMAGE"
    
    docker push "$LOGGER_IMAGE"
    
    if [ $? -eq 0 ]; then
        log_success "Logger image pushed successfully"
    else
        log_error "Failed to push logger image"
        exit 1
    fi
}

# Main execution
case $COMMAND in
    build-all)
        log_info "Building all Docker images..."
        build_server
        build_logger
        log_success "All images built successfully"
        ;;
    build-server)
        build_server
        ;;
    build-logger)
        build_logger
        ;;
    push-all)
        docker_login
        log_info "Pushing all Docker images..."
        push_server
        push_logger
        log_success "All images pushed successfully"
        ;;
    push-server)
        docker_login
        push_server
        ;;
    push-logger)
        docker_login
        push_logger
        ;;
    build-push-all)
        log_info "Building and pushing all Docker images..."
        build_server
        build_logger
        docker_login
        push_server
        push_logger
        log_success "All images built and pushed successfully"
        ;;
    login)
        docker_login
        ;;
    help)
        print_usage
        ;;
    *)
        log_error "Unknown command: $COMMAND"
        print_usage
        exit 1
        ;;
esac

# Print summary
echo ""
log_info "Summary:"
echo "  Server Image: $SERVER_IMAGE"
echo "  Logger Image: $LOGGER_IMAGE"
echo "  Tag: $TAG"