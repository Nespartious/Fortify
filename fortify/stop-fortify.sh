#!/bin/bash
# Stop all Fortify services

RUNTIME_DIR="/tmp/fortify"

echo "Stopping Fortify services..."

# Stop all services
for pid_file in "${RUNTIME_DIR}"/*.pid; do
    if [ -f "$pid_file" ]; then
        pid=$(cat "$pid_file")
        service=$(basename "$pid_file" .pid)
        if ps -p $pid > /dev/null 2>&1; then
            echo "→ Stopping $service (PID: $pid)"
            kill $pid 2>/dev/null
        fi
        rm -f "$pid_file"
    fi
done

# Kill any remaining Fortify processes
pkill -f "fortify-" 2>/dev/null || true

echo "✓ All Fortify services stopped"
