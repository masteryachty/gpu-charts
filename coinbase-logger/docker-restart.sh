#!/bin/bash

# Docker-compatible restart script for coinbase-logger
# This script restarts the container daily at midnight

# Configuration
CONTAINER_NAME="coinbase-logger"
LOG_FILE="/var/log/coinbase-logger-restart.log"

# Function to log messages
log_message() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $1" >> "$LOG_FILE"
}

# Function to restart the container
restart_container() {
    log_message "Starting daily restart process"
    
    # Check if container exists
    if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        log_message "Container ${CONTAINER_NAME} found, restarting..."
        
        # Restart the container
        if docker-compose restart "${CONTAINER_NAME}"; then
            log_message "Container ${CONTAINER_NAME} restarted successfully"
        else
            log_message "ERROR: Failed to restart container ${CONTAINER_NAME}"
            exit 1
        fi
    else
        log_message "Container ${CONTAINER_NAME} not found, starting fresh..."
        
        # Start the container using docker-compose
        if docker-compose up -d; then
            log_message "Container ${CONTAINER_NAME} started successfully"
        else
            log_message "ERROR: Failed to start container ${CONTAINER_NAME}"
            exit 1
        fi
    fi
}

# Main execution
if [ "$1" == "install-cron" ]; then
    # Install cron job for daily restart at midnight
    SCRIPT_PATH="$(readlink -f "$0")"
    CRON_JOB="0 0 * * * ${SCRIPT_PATH} >> ${LOG_FILE} 2>&1"
    
    # Check if cron job already exists
    if crontab -l 2>/dev/null | grep -q "${SCRIPT_PATH}"; then
        echo "Cron job already exists"
    else
        # Add cron job
        (crontab -l 2>/dev/null; echo "${CRON_JOB}") | crontab -
        echo "Cron job installed: ${CRON_JOB}"
        log_message "Cron job installed for daily restart"
    fi
elif [ "$1" == "remove-cron" ]; then
    # Remove cron job
    SCRIPT_PATH="$(readlink -f "$0")"
    crontab -l 2>/dev/null | grep -v "${SCRIPT_PATH}" | crontab -
    echo "Cron job removed"
    log_message "Cron job removed"
else
    # Execute restart
    restart_container
fi