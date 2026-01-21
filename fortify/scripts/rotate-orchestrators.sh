#!/bin/bash
# Orchestrator Rotation Script
# Burns compromised orchestrators and spawns new ones

set -e

echo "=== Orchestrator Rotation ==="

# Load configuration
CONFIG_FILE="${CONFIG_FILE:-/etc/fortify/fortify.toml}"

if [ ! -f "$CONFIG_FILE" ]; then
    echo "ERROR: Config file not found: $CONFIG_FILE"
    exit 1
fi

# Check if running as fortify user
if [ "$(whoami)" != "fortify" ] && [ "$EUID" -ne 0 ]; then
    echo "ERROR: Must run as root or fortify user"
    exit 1
fi

# Get list of running orchestrators
echo "Finding running orchestrators..."
ORCHESTRATOR_PIDS=$(pgrep -f fortify-orchestrator || true)

if [ -z "$ORCHESTRATOR_PIDS" ]; then
    echo "No running orchestrators found"
    exit 0
fi

echo "Found orchestrators: $ORCHESTRATOR_PIDS"

# For each orchestrator, trigger graceful shutdown
for PID in $ORCHESTRATOR_PIDS; do
    echo "Stopping orchestrator PID $PID..."
    kill -TERM $PID
    
    # Wait up to 30 seconds for graceful shutdown
    TIMEOUT=30
    while kill -0 $PID 2>/dev/null && [ $TIMEOUT -gt 0 ]; do
        sleep 1
        TIMEOUT=$((TIMEOUT - 1))
    done
    
    # Force kill if still running
    if kill -0 $PID 2>/dev/null; then
        echo "  Force killing PID $PID"
        kill -KILL $PID
    else
        echo "  Stopped gracefully"
    fi
done

# Controller will detect shutdown and spawn new orchestrators
echo ""
echo "Orchestrators stopped. Controller will spawn replacements."
echo "Check status: systemctl status fortify-orchestrator"
