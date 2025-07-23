#!/bin/bash

# Simple run script for coinbase-logger with configurable data path

# Default data path
DEFAULT_DATA_PATH="./data"

# Use provided path or default
DATA_PATH="${1:-$DEFAULT_DATA_PATH}"

# Display configuration
echo "Starting coinbase-logger with data path: $DATA_PATH"

# Ensure docker-compose is available
if ! command -v docker-compose &> /dev/null; then
    echo "Error: docker-compose is not installed"
    exit 1
fi

# Run with specified data path
DATA_PATH="$DATA_PATH" docker-compose up -d

# Check if started successfully
if [ $? -eq 0 ]; then
    echo "Coinbase logger started successfully!"
    echo "View logs with: docker-compose logs -f coinbase-logger"
else
    echo "Failed to start coinbase logger"
    exit 1
fi