#!/bin/bash

# Define paths
PROGRAM_PATH="/home/xander/coinbase-logger/target/release/coinbase-logger"
PID_FILE="/tmp/my_program.pid"

# Get current time in HH:MM format
CURRENT_TIME=$(date +"%H:%M")

# Check if the program is running
if [ -f "$PID_FILE" ]; then
    OLD_PID=$(cat $PID_FILE)
    if ps -p $OLD_PID > /dev/null; then
        # If it's midnight, force a restart
        if [ "$CURRENT_TIME" == "00:00" ]; then
            echo "Midnight restart triggered. Killing old process..."
            kill $OLD_PID
        else
            echo "Program is already running with PID $OLD_PID. Exiting..."
            exit 0
        fi
    fi
fi

# Start a new instance and save its PID
$PROGRAM_PATH &
NEW_PID=$!
echo $NEW_PID > $PID_FILE

echo "New instance started with PID $NEW_PID."



