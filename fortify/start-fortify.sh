#!/bin/bash
# Fortify Deployment Script
# Starts all components with proper configuration

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

FORTIFY_ROOT="/home/shadowbox/Fortify/Fortify/fortify"
RUNTIME_DIR="/tmp/fortify"
BIN_DIR="${FORTIFY_ROOT}/target/release"

echo -e "${GREEN}═══════════════════════════════════════${NC}"
echo -e "${GREEN}     Fortify DDoS Protection System    ${NC}"
echo -e "${GREEN}═══════════════════════════════════════${NC}"
echo ""

#===============================================================================
# PRE-START CONFLICT DETECTION & RESOLUTION
#===============================================================================
echo -e "${YELLOW}→${NC} Checking for deployment conflicts..."

# Check 1: Old systemd services
if systemctl is-active --quiet fortify 2>/dev/null || systemctl is-active --quiet fortifyd 2>/dev/null; then
    echo -e "${YELLOW}⚠${NC} Found active old systemd service(s) - stopping..."
    sudo systemctl stop fortify 2>/dev/null || true
    sudo systemctl stop fortifyd 2>/dev/null || true
    echo -e "${GREEN}✓${NC} Old services stopped"
fi

# Check 2: Port conflicts
PORTS_IN_USE=$(netstat -tuln 2>/dev/null | grep -E ":(808[0-9]|8090)" | awk '{print $4}' | awk -F: '{print $NF}' | sort -u || true)
if [ -n "$PORTS_IN_USE" ]; then
    echo -e "${YELLOW}⚠${NC} Resolving port conflicts..."
    echo "$PORTS_IN_USE" | while read -r port; do
        sudo lsof -t -i :$port 2>/dev/null | xargs -r sudo kill -9 2>/dev/null || true
    done
    echo -e "${GREEN}✓${NC} Port conflicts resolved"
fi

# Check 3: Existing Fortify processes
if pgrep -f 'fortify-|target/release/fortify' >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠${NC} Terminating existing Fortify processes..."
    pkill -9 -f 'fortify-' 2>/dev/null || true
    pkill -9 -f 'target/release/fortify' 2>/dev/null || true
    sleep 1
    echo -e "${GREEN}✓${NC} Existing processes terminated"
fi

# Check 4: Stale PID files
if [ -d "/tmp/fortify" ] && [ -n "$(find /tmp/fortify -name "*.pid" 2>/dev/null)" ]; then
    rm -f /tmp/fortify/*.pid 2>/dev/null || true
    echo -e "${GREEN}✓${NC} Cleaned up stale PID files"
fi

echo -e "${GREEN}✓${NC} Pre-start checks complete\n"

#===============================================================================
# INITIALIZATION
#===============================================================================

# Generate secret key if not exists
if [ ! -f "${RUNTIME_DIR}/config/secret.key" ]; then
    echo -e "${YELLOW}→${NC} Generating secret key..."
    openssl rand -hex 32 > "${RUNTIME_DIR}/config/secret.key"
    echo -e "${GREEN}✓${NC} Secret key generated"
fi

SECRET_KEY=$(cat "${RUNTIME_DIR}/config/secret.key")

# Check if Tor is running
if ! pgrep -x tor > /dev/null; then
    echo -e "${YELLOW}⚠${NC} Tor is not running. Starting Tor..."
    sudo systemctl start tor || echo -e "${RED}✗${NC} Failed to start Tor (continuing anyway)"
fi

# Kill any existing Fortify processes
echo -e "${YELLOW}→${NC} Stopping any existing Fortify processes..."
pkill -f "fortify-" 2>/dev/null || true
sleep 2

# Start services in order
echo ""
echo -e "${GREEN}Starting services...${NC}"
echo ""

# 1. Controller
echo -e "${YELLOW}→${NC} Starting Controller..."
cd "${RUNTIME_DIR}"
SECRET_KEY="${SECRET_KEY}" \
    nohup "${BIN_DIR}/fortify-controller" \
    > "${RUNTIME_DIR}/logs/controller.log" 2>&1 &
echo $! > "${RUNTIME_DIR}/controller.pid"
echo -e "${GREEN}✓${NC} Controller started (PID: $(cat ${RUNTIME_DIR}/controller.pid))"
sleep 2

# 2. Orchestrator
echo -e "${YELLOW}→${NC} Starting Orchestrator..."
SECRET_KEY="${SECRET_KEY}" \
    ORCH_BIND_ADDR="0.0.0.0:8080" \
    GATE_ADDRESS="http://127.0.0.1:8081" \
    TOR_CONTROL_ADDR="127.0.0.1:9051" \
    nohup "${BIN_DIR}/fortify-orchestrator" \
    > "${RUNTIME_DIR}/logs/orchestrator.log" 2>&1 &
echo $! > "${RUNTIME_DIR}/orchestrator.pid"
echo -e "${GREEN}✓${NC} Orchestrator started (PID: $(cat ${RUNTIME_DIR}/orchestrator.pid))"
sleep 3

# 3. Gate
echo -e "${YELLOW}→${NC} Starting Gate..."
SECRET_KEY="${SECRET_KEY}" \
    GATE_BIND_ADDR="0.0.0.0:8081" \
    GATE_STATIC_DIR="${RUNTIME_DIR}/assets/html" \
    GATE_MAX_CONCURRENT="100" \
    nohup "${BIN_DIR}/fortify-gate" \
    > "${RUNTIME_DIR}/logs/gate.log" 2>&1 &
echo $! > "${RUNTIME_DIR}/gate.pid"
echo -e "${GREEN}✓${NC} Gate started (PID: $(cat ${RUNTIME_DIR}/gate.pid))"
sleep 2

# 4. Healthy Nodes (2 instances)
echo -e "${YELLOW}→${NC} Starting Healthy Nodes..."
for i in 1 2; do
    PORT=$((8090 + i))
    SECRET_KEY="${SECRET_KEY}" \
        NODE_MODE="healthy" \
        BIND_ADDR="0.0.0.0:${PORT}" \
        nohup "${BIN_DIR}/fortify-node" \
        > "${RUNTIME_DIR}/logs/node-healthy-${i}.log" 2>&1 &
    echo $! > "${RUNTIME_DIR}/node-healthy-${i}.pid"
done
echo -e "${GREEN}✓${NC} Healthy nodes started (ports 8091-8092)"
sleep 2

# 5. Threat Node
echo -e "${YELLOW}→${NC} Starting Threat Node..."
SECRET_KEY="${SECRET_KEY}" \
    NODE_MODE="threat" \
    BIND_ADDR="0.0.0.0:8093" \
    nohup "${BIN_DIR}/fortify-node" \
    > "${RUNTIME_DIR}/logs/node-threat.log" 2>&1 &
echo $! > "${RUNTIME_DIR}/node-threat.pid"
echo -e "${GREEN}✓${NC} Threat node started (port 8093)"
sleep 2

# 6. HTTP Proxy
echo -e "${YELLOW}→${NC} Starting HTTP Proxy..."
SECRET_KEY="${SECRET_KEY}" \
    PROXY_BIND_ADDR="0.0.0.0:8082" \
    HEALTHY_NODES="127.0.0.1:8091,127.0.0.1:8092" \
    THREAT_NODES="127.0.0.1:8093" \
    GATE_ADDRESS="http://127.0.0.1:8081" \
    nohup "${BIN_DIR}/fortify-http" \
    > "${RUNTIME_DIR}/logs/http-proxy.log" 2>&1 &
echo $! > "${RUNTIME_DIR}/http-proxy.pid"
echo -e "${GREEN}✓${NC} HTTP Proxy started (port 8082)"

sleep 3

# Check all services
echo ""
echo -e "${GREEN}═══════════════════════════════════════${NC}"
echo -e "${GREEN}     Service Status Check              ${NC}"
echo -e "${GREEN}═══════════════════════════════════════${NC}"
echo ""

check_service() {
    local name=$1
    local pid_file=$2
    local port=$3
    
    if [ -f "${pid_file}" ]; then
        local pid=$(cat "${pid_file}")
        if ps -p $pid > /dev/null 2>&1; then
            echo -e "${GREEN}✓${NC} ${name} (PID: ${pid}, Port: ${port})"
            return 0
        else
            echo -e "${RED}✗${NC} ${name} (failed to start)"
            return 1
        fi
    else
        echo -e "${RED}✗${NC} ${name} (no PID file)"
        return 1
    fi
}

check_service "Controller      " "${RUNTIME_DIR}/controller.pid" "8000"
check_service "Orchestrator    " "${RUNTIME_DIR}/orchestrator.pid" "8080"
check_service "Gate            " "${RUNTIME_DIR}/gate.pid" "8081"
check_service "HTTP Proxy      " "${RUNTIME_DIR}/http-proxy.pid" "8082"
check_service "Healthy Node 1  " "${RUNTIME_DIR}/node-healthy-1.pid" "8091"
check_service "Healthy Node 2  " "${RUNTIME_DIR}/node-healthy-2.pid" "8092"
check_service "Threat Node     " "${RUNTIME_DIR}/node-threat.pid" "8093"

echo ""
echo -e "${GREEN}═══════════════════════════════════════${NC}"
echo -e "${GREEN}     Deployment Complete!              ${NC}"
echo -e "${GREEN}═══════════════════════════════════════${NC}"
echo ""
echo -e "Logs: ${RUNTIME_DIR}/logs/"
echo -e "HTTP Proxy: ${GREEN}http://127.0.0.1:8082${NC}"
echo -e "Gate: ${GREEN}http://127.0.0.1:8081${NC}"
echo -e "Orchestrator: ${GREEN}http://127.0.0.1:8080${NC}"
echo ""
echo -e "To view logs:"
echo -e "  tail -f ${RUNTIME_DIR}/logs/http-proxy.log"
echo -e "  tail -f ${RUNTIME_DIR}/logs/gate.log"
echo ""
echo -e "To stop all services:"
echo -e "  ./stop-fortify.sh"
echo ""
