#!/bin/bash
# Release Run Script for Fortify
# This script starts all Fortify components using release binaries

set -e

echo "=== Fortify Release Startup ==="

PROJECT_ROOT="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$PROJECT_ROOT"

# Stop any existing Fortify processes
echo "Stopping existing Fortify processes..."
pkill -f "fortify-" 2>/dev/null || true
sleep 1

# Set up temporary directory
mkdir -p /tmp/fortify/config
mkdir -p /tmp/fortify/logs

# Create minimal configuration
CONFIG_PATH="/tmp/fortify/config/fortify.toml"
if [ ! -f "$CONFIG_PATH" ]; then
    echo "Creating default configuration..."
    cat > "$CONFIG_PATH" <<'EOF'
[controller]
bind_address = "127.0.0.1:9090"
health_check_interval = 10
min_orchestrators = 1
max_orchestrators = 2
min_healthy_nodes = 1
max_healthy_nodes = 3
min_threat_nodes = 0
max_threat_nodes = 1

[orchestrator]
bind_address = "127.0.0.1:8080"
real_onion_address = "your-real-service.onion"
real_service_port = 80

[vanity]
enabled = false
prefix = ""
timeout_seconds = 300

[node]
bind_base = "127.0.0.1:8083"
backend_address = "http://127.0.0.1:9000"
violation_threshold = 3
rate_limit_requests = 20
rate_limit_window_seconds = 10

[gate]
bind_address = "127.0.0.1:8081"
max_concurrent = 100
pow_difficulty = 20
verification_timeout = 300

[proxy]
bind_address = "0.0.0.0:8082"
max_concurrent = 1000
EOF
fi

# Set environment variables
export SECRET_KEY="fortify-secret-key-$(date +%s)"
export RUST_LOG=${RUST_LOG:-info}
export PATH="$PROJECT_ROOT/target/release:$PATH"

# Check if release binaries exist
if [ ! -f "target/release/fortify-controller" ]; then
    echo "Error: Release binaries not found. Run 'cargo build --release' first."
    exit 1
fi

echo ""
echo "=== Starting Backend Proxy (Mock) ==="
# Start a simple HTTP server as a mock backend on port 9000
python3 -m http.server 9000 --directory /tmp/fortify &
BACKEND_PID=$!
echo "Backend Mock PID: $BACKEND_PID (serving test content on port 9000)"

sleep 2

echo ""
echo "=== Starting Fortify Controller ==="
echo "Controller will manage all services (Gate, Proxy, Nodes, Orchestrators)"
echo ""

# Start controller (it will spawn all other services)
RUST_LOG=${RUST_LOG:-info} ./target/release/fortify-controller &
CONTROLLER_PID=$!

echo "Controller PID: $CONTROLLER_PID"
echo ""
echo "=== Services Starting ==="
echo "Waiting for services to come online..."

# Wait for services to be ready
for i in {1..15}; do
    if curl -s http://127.0.0.1:8082/health > /dev/null 2>&1; then
        echo "✓ HTTP Proxy is ready on port 8082"
        break
    fi
    if [ $i -eq 15 ]; then
        echo "✗ HTTP Proxy failed to start (check logs)"
    fi
    sleep 1
done

echo ""
echo "=== Fortify is Running ==="
echo ""
echo "Service URLs:"
echo "  HTTP Proxy:       http://127.0.0.1:8082"
echo "  Gate (CAPTCHA):   http://127.0.0.1:8081"
echo "  Controller:       http://127.0.0.1:9090"
echo "  Orchestrator:     http://127.0.0.1:8080"
echo ""
echo "Test access:"
echo "  curl http://127.0.0.1:8082/"
echo ""
echo "View logs:"
echo "  tail -f /tmp/fortify/logs/*.log"
echo ""
echo "Stop services:"
echo "  pkill -f fortify-"
echo ""
echo "Press Ctrl+C to stop all services..."

# Wait for Ctrl+C
trap "echo 'Stopping services...'; pkill -f 'fortify-'; kill $BACKEND_PID 2>/dev/null || true; echo 'Stopped.'; exit 0" INT TERM

wait $CONTROLLER_PID
