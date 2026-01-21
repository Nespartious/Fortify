#!/bin/bash
# Mirror Burn Script
# Emergency burn of compromised mirror/orchestrator

set -e

if [ $# -lt 1 ]; then
    echo "Usage: $0 <orchestrator_pid|all>"
    echo ""
    echo "Burns a specific orchestrator or all orchestrators"
    echo ""
    echo "Examples:"
    echo "  $0 12345        # Burn orchestrator PID 12345"
    echo "  $0 all          # Burn all orchestrators (emergency)"
    exit 1
fi

TARGET=$1

echo "=== MIRROR BURN INITIATED ==="
echo "Target: $TARGET"
echo ""

if [ "$TARGET" = "all" ]; then
    echo "WARNING: This will burn ALL orchestrators"
    read -p "Are you sure? (type YES): " CONFIRM
    
    if [ "$CONFIRM" != "YES" ]; then
        echo "Aborted"
        exit 1
    fi
    
    echo "Burning all orchestrators..."
    PIDS=$(pgrep -f fortify-orchestrator || true)
    
    if [ -z "$PIDS" ]; then
        echo "No orchestrators running"
        exit 0
    fi
    
    for PID in $PIDS; do
        echo "  Killing PID $PID"
        kill -KILL $PID 2>/dev/null || true
    done
    
    # Clean up Tor hidden service directories
    echo "Cleaning Tor hidden service directories..."
    rm -rf /var/lib/tor/fortify-orchestrator* 2>/dev/null || true
    
    echo "All orchestrators burned"
    echo "Restart with: systemctl restart fortify-orchestrator"
    
else
    # Burn specific PID
    if ! kill -0 "$TARGET" 2>/dev/null; then
        echo "ERROR: Process $TARGET not found or not accessible"
        exit 1
    fi
    
    # Verify it's an orchestrator
    PROCESS_NAME=$(ps -p "$TARGET" -o comm= || true)
    if [[ "$PROCESS_NAME" != *"orchestrator"* ]]; then
        echo "WARNING: PID $TARGET doesn't appear to be an orchestrator"
        read -p "Continue anyway? (y/N): " CONFIRM
        if [ "$CONFIRM" != "y" ]; then
            echo "Aborted"
            exit 1
        fi
    fi
    
    echo "Burning orchestrator PID $TARGET"
    kill -KILL "$TARGET"
    
    echo "Orchestrator $TARGET burned"
    echo "Controller will spawn replacement"
fi

echo ""
echo "=== BURN COMPLETE ==="
