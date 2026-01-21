#!/bin/bash
# ============================================================================
# DEPRECATED: This script is no longer the official deployment method
# ============================================================================
# 
# Please use the TUI deployment wizard instead:
#   ./target/release/fortify
#
# The TUI provides:
#   - Unified deployment workflow
#   - Configuration wizard
#   - Real-time log monitoring
#   - Mirror status tracking
#   - One-click export of mirror addresses
#
# This dev-run.sh script is kept for backward compatibility but may be
# removed in future versions. It is no longer maintained.
#
# ============================================================================
#
# Legacy Development Run Script
# Runs Fortify components locally for testing
#
# Usage: ./dev-run.sh [--wipe]
#   --wipe   Completely reset all Fortify state before starting

echo ""
echo "╔══════════════════════════════════════════════════════════════════╗"
echo "║                                                                  ║"
echo "║  ⚠️  WARNING: dev-run.sh is DEPRECATED                          ║"
echo "║                                                                  ║"
echo "║  Please use the TUI deployment wizard instead:                  ║"
echo "║    ./target/release/fortify                                      ║"
echo "║                                                                  ║"
echo "║  This script may be removed in future versions.                 ║"
echo "║                                                                  ║"
echo "╚══════════════════════════════════════════════════════════════════╝"
echo ""
sleep 3

set -e

WIPE_MODE=0
if [[ "$1" == "--wipe" || "$1" == "-w" ]]; then
    WIPE_MODE=1
fi

echo "=== Fortify Development Environment ==="

# Get the script's directory and compute project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

# Change to project root to ensure all relative paths work
cd "$PROJECT_ROOT"

# Ensure locally built binaries are available to spawned services
export PATH="$PROJECT_ROOT/target/debug:$PATH"

# Stop any lingering Fortify binaries to avoid port conflicts
if pgrep -f "fortify-" >/dev/null 2>&1; then
    echo "Stopping existing Fortify processes..."
    pkill -f "fortify-" || true
    sleep 1
fi

CONFIG_PATH="/tmp/fortify/config/fortify.toml"
TOR_BASE=/tmp/fortify/tor

# --- STATE MANAGEMENT ---
# Check if a previous run state exists
if [ -d "/tmp/fortify" ]; then
    if [ "$WIPE_MODE" -eq 1 ]; then
        echo "Wiping existing state (--wipe flag)..."
        response="w"
    else
        echo "Found existing Fortify state in /tmp/fortify."
        echo ""
        echo "Options:"
        echo "  [R]esume - Keep existing keys and configuration"
        echo "  [W]ipe   - Delete everything and start fresh"
        echo ""
        read -p "Choose [R/w]: " response
        response=${response,,} # tolower
    fi
    
    if [[ "$response" == "w" || "$response" == "wipe" ]]; then
        echo "Wiping existing state..."
        
        # Kill everything first
        pkill -f "fortify-" 2>/dev/null || true
        pkill -f "onion_proxy.py" 2>/dev/null || true
        
        if [ -f "$TOR_BASE/tor.pid" ]; then
             kill $(cat "$TOR_BASE/tor.pid") 2>/dev/null || true
        fi
        
        # Also kill any tor on our ports
        fuser -k 9150/tcp 2>/dev/null || true
        fuser -k 9151/tcp 2>/dev/null || true
        
        sleep 1
        
        # Clean directories completely
        rm -rf /tmp/fortify
        echo "State wiped."
    else
        echo "Resuming session..."
    fi
fi
# ------------------------

# Always build to ensure changes are picked up
echo "Building workspace..."
cargo build --workspace

# Setup vanguards addon
echo ""
echo "=== Setting up Vanguards Addon ==="
setup_vanguards() {
    # Check if vanguards is already installed and working
    if command -v vanguards &> /dev/null && vanguards --help &>/dev/null 2>&1; then
        echo "✓ Vanguards already installed: $(which vanguards)"
        return 0
    fi
    
    # Check ~/.local/bin
    if [ -f "$HOME/.local/bin/vanguards" ] && "$HOME/.local/bin/vanguards" --help &>/dev/null 2>&1; then
        echo "✓ Vanguards found at ~/.local/bin/vanguards"
        export PATH="$HOME/.local/bin:$PATH"
        return 0
    fi
    
    # Check if venv exists from previous run with working vanguards
    VENV_DIR="/tmp/fortify/venv"
    if [ -f "$VENV_DIR/bin/vanguards" ] && "$VENV_DIR/bin/vanguards" --help &>/dev/null 2>&1; then
        echo "✓ Vanguards found in Fortify venv"
        export PATH="$VENV_DIR/bin:$PATH"
        return 0
    fi
    
    # Check if Python3 is available
    if ! command -v python3 &> /dev/null; then
        echo "✗ Python3 not found - vanguards will not be available"
        echo "  Install with: sudo apt install python3 python3-pip python3-venv"
        return 1
    fi
    
    # Check Python version - vanguards has issues with Python 3.12+
    PYTHON_VERSION=$(python3 -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
    PYTHON_MAJOR=$(echo $PYTHON_VERSION | cut -d. -f1)
    PYTHON_MINOR=$(echo $PYTHON_VERSION | cut -d. -f2)
    
    echo "Installing vanguards addon (Python $PYTHON_VERSION)..."
    
    if [ "$PYTHON_MAJOR" -ge 3 ] && [ "$PYTHON_MINOR" -ge 12 ]; then
        # Python 3.12+ needs patched vanguards
        echo "Note: Python 3.12+ detected, installing patched vanguards from git..."
        install_vanguards_from_git
        return $?
    fi
    
    # Try pip install first (user mode)
    if pip3 install --user vanguards 2>/dev/null; then
        echo "✓ Vanguards installed via pip (user mode)"
        export PATH="$HOME/.local/bin:$PATH"
        return 0
    fi
    
    # If that fails (externally-managed-environment), try venv
    if python3 -m venv "$VENV_DIR" 2>/dev/null; then
        echo "Creating Fortify virtual environment for vanguards..."
        if "$VENV_DIR/bin/pip" install vanguards 2>/dev/null; then
            echo "✓ Vanguards installed in Fortify venv"
            export PATH="$VENV_DIR/bin:$PATH"
            return 0
        fi
    fi
    
    # Fallback to git install
    install_vanguards_from_git
    return $?
}

install_vanguards_from_git() {
    VENV_DIR="/tmp/fortify/venv"
    VANGUARDS_GIT="/tmp/fortify/vanguards-src"
    
    echo "Cloning vanguards from GitHub..."
    rm -rf "$VANGUARDS_GIT"
    if ! git clone --quiet https://github.com/mikeperry-tor/vanguards.git "$VANGUARDS_GIT" 2>/dev/null; then
        echo "✗ Failed to clone vanguards repository"
        return 1
    fi
    
    # Patch for Python 3.12+ compatibility
    echo "Applying Python 3.12 compatibility patch..."
    CONFIGPY="$VANGUARDS_GIT/src/vanguards/config.py"
    if [ -f "$CONFIGPY" ]; then
        # Replace SafeConfigParser with ConfigParser (they're equivalent in modern Python)
        sed -i 's/SafeConfigParser/ConfigParser/g' "$CONFIGPY"
        sed -i 's/from configparser import ConfigParser, Error/from configparser import ConfigParser as ConfigParser, Error/g' "$CONFIGPY" 2>/dev/null || true
    fi
    
    # Create venv and install
    mkdir -p "$VENV_DIR"
    if python3 -m venv "$VENV_DIR" 2>/dev/null; then
        if "$VENV_DIR/bin/pip" install -e "$VANGUARDS_GIT" 2>/dev/null; then
            if "$VENV_DIR/bin/vanguards" --help &>/dev/null 2>&1; then
                echo "✓ Vanguards installed from git (patched for Python 3.12+)"
                export PATH="$VENV_DIR/bin:$PATH"
                return 0
            fi
        fi
    fi
    
    echo "⚠ Could not install vanguards - guard protection will be reduced"
    echo "  For Python 3.12+, you may need to use an older Python version"
    return 1
}

setup_vanguards
VANGUARDS_AVAILABLE=$?
echo ""


# Create temporary directories
mkdir -p /tmp/fortify/{log,run,config}

# Copy example config
if [ ! -f "$CONFIG_PATH" ]; then
    cp config/fortify.example.toml "$CONFIG_PATH"
    echo "Config copied to $CONFIG_PATH"
    # Don't exit - we'll prompt for the onion address below
fi

# Ensure the protected onion address has been configured
if grep -q "your-real-service-xxxxxxxxxxxxxx.onion" "$CONFIG_PATH"; then
    echo "Config has placeholder address."
    # Will be handled by python script logic or interactive prompt below
fi

COMPONENT_INFO=$(
python3 - "$CONFIG_PATH" <<'PY'
import sys
from pathlib import Path
try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore

config_path = Path(sys.argv[1])
data = tomllib.loads(config_path.read_text())

def fetch(section, key, default):
    return str(data.get(section, {}).get(key, default))

orch = fetch(
    "orchestrator",
    "public_bind_address",
    fetch("orchestrator", "bind_address", "127.0.0.1:8080"),
)
gate = fetch("gate", "bind_address", "127.0.0.1:8081")
proxy = fetch("http_proxy", "bind_address", "127.0.0.1:8082")
onion = fetch("service", "real_onion_address", "")
port = fetch("service", "real_service_port", 80)

controller = fetch("controller", "bind_address", "127.0.0.1:7000")
node_base = fetch("node", "bind_base", "127.0.0.1:9100")
node_backend = fetch("node", "backend_address", "http://127.0.0.1:9000")

print("|".join([orch, gate, proxy, onion, port, controller, node_base, node_backend]))
PY
)

if [ $? -ne 0 ]; then
    echo "Failed to parse $CONFIG_PATH; ensure python3 with tomllib/tomli is available."
    exit 1
fi

IFS='|' read -r ORCH_ADDR GATE_ADDR PROXY_ADDR REAL_ONION REAL_PORT CONTROLLER_ADDR NODE_BASE NODE_BACKEND <<< "$COMPONENT_INFO"

if [ -z "$REAL_ONION" ] || [ "$REAL_ONION" = "None" ] || [[ "$REAL_ONION" == *"your-real-service"* ]]; then
    echo "NO PROTECTED ONION ADDRESS CONFIGURED."
    echo ""
    echo "Enter the Onion Address of the service you want to protect."
    echo "Example: facebookcorewwwi.onion"
    read -p "Onion Address: " USER_ONION
    
    if [ -z "$USER_ONION" ]; then
        echo "Error: You must provide an onion address."
        exit 1
    fi
    
    read -p "Service Port (default 80): " USER_PORT
    USER_PORT=${USER_PORT:-80}
    
    # Update config file in place
    # sed -i is slightly different on BSD/Mac, but assuming Linux from context
    sed -i "s|real_onion_address = .*|real_onion_address = \"$USER_ONION\"|" "$CONFIG_PATH"
    sed -i "s|real_service_port = .*|real_service_port = $USER_PORT|" "$CONFIG_PATH"
    
    echo "Updated $CONFIG_PATH with $USER_ONION:$USER_PORT"
    
    REAL_ONION="$USER_ONION"
    REAL_PORT="$USER_PORT"
fi

if [ -z "$CONTROLLER_ADDR" ] || [ "$CONTROLLER_ADDR" = "None" ]; then
    echo "controller.bind_address is missing from $CONFIG_PATH"
    exit 1
fi

if [ -z "$NODE_BASE" ] || [ "$NODE_BASE" = "None" ]; then
    echo "node.bind_base is missing from $CONFIG_PATH"
    exit 1
fi

if ! command -v tor >/dev/null 2>&1; then
    echo "tor binary not found. Install tor (e.g., sudo apt install tor) before running this script."
    exit 1
fi

TOR_BASE=/tmp/fortify/tor
TOR_DATA_DIR="$TOR_BASE/data"
TOR_CONTROL_ADDR="127.0.0.1:9151"
TOR_SOCKS_ADDR="127.0.0.1:9150"
TOR_PID_FILE="$TOR_BASE/tor.pid"
TOR_TORRC="$TOR_BASE/torrc"
TOR_LOG="$TOR_BASE/tor.log"
MANAGED_TOR=0

mkdir -p "$TOR_BASE"

cat > "$TOR_TORRC" <<EOF
DataDirectory $TOR_DATA_DIR
ControlPort $TOR_CONTROL_ADDR
SocksPort $TOR_SOCKS_ADDR
CookieAuthentication 1
Log notice file $TOR_LOG
EOF

if [ -f "$TOR_PID_FILE" ] && kill -0 "$(cat "$TOR_PID_FILE")" 2>/dev/null; then
    echo "Using existing Fortify Tor daemon (PID $(cat "$TOR_PID_FILE"))"
else
    # Double check no lingering tor processes on that port if we think we started clean
    if lsof -i :9150 >/dev/null 2>&1; then
        echo "Warning: Port 9150 is in use, attempting to kill likely stale Tor process..."
        fuser -k 9150/tcp || true
    fi

    echo "Starting dedicated Tor daemon..."
    tor -f "$TOR_TORRC" --RunAsDaemon 1 --PidFile "$TOR_PID_FILE"
    MANAGED_TOR=1
fi

echo "Waiting for Tor control cookie..."
for _ in {1..30}; do
    if [ -f "$TOR_DATA_DIR/control_auth_cookie" ]; then
        break
    fi
    sleep 1
done

if [ ! -f "$TOR_DATA_DIR/control_auth_cookie" ]; then
    echo "Tor control cookie not found; check logs at $TOR_LOG"
    exit 1
fi

export TOR_CONTROL_ADDR="$TOR_CONTROL_ADDR"
export TOR_COOKIE_PATH="$TOR_DATA_DIR/control_auth_cookie"
export ORCH_BIND_ADDR="$ORCH_ADDR"
export GATE_ADDRESS="http://$GATE_ADDR"
export CONTROLLER_BIND_ADDR="$CONTROLLER_ADDR"
export GATE_BIND_ADDR="$GATE_ADDR"
export PROXY_BIND_ADDR="$PROXY_ADDR"
export NODE_BIND_BASE="$NODE_BASE"
export NODE_BACKEND_ADDR="$NODE_BACKEND"
export SECRET_KEY="${SECRET_KEY:-fortify-secret-key}"

# Vanguards configuration (controller will auto-start vanguards if available)
export VANGUARDS_ENABLED=true
export VANGUARDS_LAYER2_GUARDS=4
export VANGUARDS_LAYER3_GUARDS=8
export VANGUARDS_CIRC_MAX_AGE_HOURS=24

# Control Panel secret path
ADMIN_PATH="/ctrl_8f7k3m9x2n4p1q6w5v0b8c"

echo "Starting Fortify components..."
echo ""

# Start backend proxy (Forward traffic to the hidden service via Tor)
# Clean the onion address (remove http:// and trailing /)
CLEAN_ONION=$(echo "$REAL_ONION" | sed -E 's|https?://||' | cut -d/ -f1)
echo "Starting backend proxy to $CLEAN_ONION:$REAL_PORT..."
# SOCKS host/port
SOCKS_HOST=$(echo $TOR_SOCKS_ADDR | cut -d: -f1)
SOCKS_PORT=$(echo $TOR_SOCKS_ADDR | cut -d: -f2)

python3 scripts/onion_proxy.py \
    --listen-port 9000 \
    --onion-addr "$CLEAN_ONION" \
    --onion-port "$REAL_PORT" \
    --socks-host "$SOCKS_HOST" \
    --socks-port "$SOCKS_PORT" &
BACKEND_PID=$!
echo "Backend Proxy PID: $BACKEND_PID"

# Start controller (it manages gate, proxy, orchestrators, and nodes)
echo "Starting controller (manages all Fortify services)..."
RUST_LOG=${RUST_LOG:-debug} ./target/debug/fortify-controller &
CONTROLLER_PID=$!
echo "  Controller PID: $CONTROLLER_PID"

echo ""
echo "=== Controller is provisioning services ==="
echo "Protected service: $REAL_ONION:$REAL_PORT"
echo ""
echo "Health & status URLs (adjust if you changed bind addresses):"
echo "  Orchestrator status: http://$ORCH_ADDR/status"
echo "  Orchestrator mirrors API: http://$ORCH_ADDR/mirrors"
echo "  Gate challenge endpoint: http://$GATE_ADDR"
echo "  HTTP proxy ingress: http://$PROXY_ADDR"
echo ""
echo "Waiting for public onion mirrors to come online..."

report_mirrors() {
    local attempt payload
    for attempt in {1..30}; do
        payload=$(curl -fsS --max-time 3 "http://$ORCH_ADDR/mirrors" 2>/dev/null || true)
        if [ -n "$payload" ]; then
            # Store mirrors for later use in summary box
            MIRROR_LIST=$(python3 - "$payload" <<'PY'
import json
import sys

try:
    data = json.loads(sys.argv[1])
except json.JSONDecodeError as exc:
    print("")
    sys.exit(1)

mirrors = data.get("mirrors", [])
if not mirrors:
    print("")
    sys.exit(1)

# Output mirrors one per line for easy parsing
for onion in mirrors:
    print(onion)
sys.exit(0)
PY
)
            if [ -n "$MIRROR_LIST" ]; then
                echo ""
                echo "=== Public Fortify Onion Mirrors ==="
                local idx=1
                while IFS= read -r onion; do
                    echo "  $idx. $onion"
                    ((idx++))
                done <<< "$MIRROR_LIST"
                echo "==================================="
                echo ""
                return 0
            fi
        fi
        sleep 2
    done
    echo "Timed out waiting for mirrors; check controller logs at target/debug/fortify-controller." >&2
    MIRROR_LIST=""
    return 1
}

report_mirrors || true

report_nodes() {
    local attempt payload
    for attempt in {1..20}; do
        payload=$(curl -fsS --max-time 3 "http://$CONTROLLER_ADDR/nodes" 2>/dev/null || true)
        if [ -n "$payload" ]; then
            if python3 - "$payload" "$GATE_ADDR" <<'PY'
import json
import sys

try:
    data = json.loads(sys.argv[1])
except json.JSONDecodeError as exc:
    print(f"Unable to parse controller /nodes payload: {exc}")
    sys.exit(1)

gate_addr = sys.argv[2] if len(sys.argv) > 2 else "127.0.0.1:8081"

nodes = data.get("nodes")
if not nodes:
    print("Controller responded but no nodes are reported yet.")
    sys.exit(1)

healthy = [n for n in nodes if n.get("mode") == "healthy"]
threat = [n for n in nodes if n.get("mode") == "threat"]

print("\n=== Fortify Node Pools ===")
print(f"\n  HEALTHY POOL ({len(healthy)} nodes):")
for node in healthy:
    status = node.get("status", "unknown")
    addr = node.get("bind_addr", "-")
    restarts = node.get("restart_count", 0)
    print(f"    • {node.get('id')}: {status} on {addr} (restarts={restarts})")

# Show Gate as the threat pool destination (Unknown/Suspicious users get routed here)
# The Gate handles captcha verification before allowing access
print(f"\n  THREAT POOL (Gate - captcha verification):")
print(f"    • Gate: http://{gate_addr}")
print(f"    └─ Unknown/Suspicious users routed here for verification")
if threat:
    print(f"\n  ADDITIONAL THREAT NODES ({len(threat)}):")
    for node in threat:
        status = node.get("status", "unknown")
        addr = node.get("bind_addr", "-")
        restarts = node.get("restart_count", 0)
        print(f"    • {node.get('id')}: {status} on {addr} (restarts={restarts})")

print("===========================\n")
sys.exit(0)
PY
            then
                return
            fi
        fi
        sleep 2
    done
    echo "Timed out waiting for node status; check controller API at http://$CONTROLLER_ADDR/." >&2
    return 1
}

report_nodes || true

# Build mirror display section
MIRROR_SECTION=""
FIRST_MIRROR=""
if [ -n "$MIRROR_LIST" ]; then
    # Get first mirror for admin panel link
    FIRST_MIRROR=$(echo "$MIRROR_LIST" | head -1)
    
    MIRROR_SECTION=$(
        echo "║  🌐 PUBLIC ONION MIRRORS                                            ║"
        while IFS= read -r onion; do
            if [ -n "$onion" ]; then
                printf "║    • %-60s ║\n" "$onion"
            fi
        done <<< "$MIRROR_LIST"
        echo "║                                                                    ║"
    )
fi

# Print summary with all addresses
echo ""
echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║        🏰  F O R T I F Y   C O N T R O L   C I T A D E L  🏰       ║"
echo "╠════════════════════════════════════════════════════════════════════╣"
echo "║                                                                    ║"
echo "║  🛡️  PROTECTED SERVICE                                             ║"
printf "║    %-64s ║\n" "$REAL_ONION:$REAL_PORT"
echo "║                                                                    ║"
echo "║  ⚡ LOCAL ENDPOINTS                                                 ║"
printf "║    HTTP Proxy:     %-46s ║\n" "http://$PROXY_ADDR"
printf "║    Gate:           %-46s ║\n" "http://$GATE_ADDR"
printf "║    Orchestrator:   %-46s ║\n" "http://$ORCH_ADDR"
printf "║    Controller:     %-46s ║\n" "http://$CONTROLLER_ADDR"
printf "║    Backend Proxy:  %-46s ║\n" "http://127.0.0.1:9000"
echo "║                                                                    ║"
if [ -n "$MIRROR_SECTION" ]; then
    echo "$MIRROR_SECTION"
fi
echo "║  🎛️  ADMIN CONTROL PANEL                                           ║"
printf "║    Local:  %-55s ║\n" "http://$PROXY_ADDR$ADMIN_PATH"
if [ -n "$FIRST_MIRROR" ]; then
printf "║    Onion:  %-55s ║\n" "http://$FIRST_MIRROR$ADMIN_PATH"
fi
echo "║                                                                    ║"
echo "║  🧅 TOR                                                             ║"
printf "║    SOCKS Proxy:    %-46s ║\n" "$TOR_SOCKS_ADDR"
printf "║    Control Port:   %-46s ║\n" "$TOR_CONTROL_ADDR"
if [ "$VANGUARDS_AVAILABLE" -eq 0 ]; then
echo "║    Vanguards:      ✓ Enabled (Guard Discovery Protection)         ║"
else
echo "║    Vanguards:      ✗ Not Available (reduced protection)           ║"
fi
echo "║                                                                    ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""
echo "The controller supervises all child services."
echo "Watch this terminal for rotating mirror announcements."
echo "Press Ctrl+C to stop everything."
echo ""

# Trap Ctrl+C and clean up resources
cleanup() {
    if [ -n "$CONTROLLER_PID" ]; then
        echo "Stopping controller..."
        kill "$CONTROLLER_PID" 2>/dev/null || true
        wait "$CONTROLLER_PID" 2>/dev/null || true
        CONTROLLER_PID=""
    fi
    if [ -n "$BACKEND_PID" ]; then
        echo "Stopping backend..."
        kill "$BACKEND_PID" 2>/dev/null || true
        BACKEND_PID=""
    fi
    # Stop any vanguards processes that may have been spawned
    if pgrep -f "vanguards" > /dev/null 2>&1; then
        echo "Stopping vanguards..."
        pkill -f "vanguards" 2>/dev/null || true
    fi
    if [ "$MANAGED_TOR" = "1" ] && [ -f "$TOR_PID_FILE" ]; then
        echo "Stopping Tor..."
        kill "$(cat "$TOR_PID_FILE")" 2>/dev/null || true
        rm -f "$TOR_PID_FILE"
    fi
    exit 0
}

trap cleanup INT TERM

# Wait for controller to exit naturally
wait $CONTROLLER_PID
CONTROLLER_PID=""
cleanup
