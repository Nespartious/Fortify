#!/bin/bash
#===============================================================================
#
#   ███████╗ ██████╗ ██████╗ ████████╗██╗███████╗██╗   ██╗
#   ██╔════╝██╔═══██╗██╔══██╗╚══██╔══╝██║██╔════╝╚██╗ ██╔╝
#   █████╗  ██║   ██║██████╔╝   ██║   ██║█████╗   ╚████╔╝ 
#   ██╔══╝  ██║   ██║██╔══██╗   ██║   ██║██╔══╝    ╚██╔╝  
#   ██║     ╚██████╔╝██║  ██║   ██║   ██║██║        ██║   
#   ╚═╝      ╚═════╝ ╚═╝  ╚═╝   ╚═╝   ╚═╝╚═╝        ╚═╝   
#
#   HEADLESS DEPLOYMENT SCRIPT
#   Version: 1.0.0
#
#   This script deploys Fortify on a fresh Ubuntu server without user
#   interaction. Edit the configuration section below, then run:
#
#       sudo ./deploy.sh
#
#   For more information: https://github.com/Nespartious/Fortify
#
#===============================================================================

set -e  # Exit on any error

#===============================================================================
#                    CONFIGURATION - EDIT THESE VALUES
#===============================================================================

#-------------------------------------------------------------------------------
# BACKEND SERVICE SETTINGS
# The service you want to protect. Fortify sits in front of this.
#-------------------------------------------------------------------------------

# The address of your real service (the one Fortify protects)
# This should be accessible from localhost only - never expose it directly!
# Examples: "http://127.0.0.1:8080", "http://127.0.0.1:9000"
BACKEND_ADDRESS="http://127.0.0.1:9000"

# Display name for your service (shown on CAPTCHA pages)
SERVICE_NAME="My Protected Service"

# Short description (shown on CAPTCHA pages)
SERVICE_DESCRIPTION="A Fortify-protected onion service"

# Welcome message on CAPTCHA page
WELCOME_MESSAGE="Please complete the verification to continue."

# Primary color for CAPTCHA pages (hex color)
PRIMARY_COLOR="#6B46C1"

#-------------------------------------------------------------------------------
# MIRROR SETTINGS  
# Mirrors are the .onion addresses users connect to. Fortify manages multiple
# mirrors and rotates/burns them to absorb attacks.
#-------------------------------------------------------------------------------

# Minimum number of active mirrors (always running)
MIN_MIRRORS=2

# Maximum mirrors allowed (hard cap)
MAX_MIRRORS=5

# Standby mirrors to maintain (ready to activate instantly)
STANDBY_MIRRORS=2

# How often to check mirror health (seconds)
ROTATION_INTERVAL_SECONDS=3600

#-------------------------------------------------------------------------------
# VANITY ADDRESS SETTINGS
# Vanity addresses make your .onion addresses start with a custom prefix
# (e.g., "fortify" -> fortifyxyz123...onion). Requires mkp224o.
#-------------------------------------------------------------------------------

# Enable vanity address generation (true/false)
# WARNING: If enabled, mkp224o will be automatically installed
VANITY_ENABLED=false

# Prefix for vanity addresses (max 5-6 chars recommended)
# Longer prefixes take exponentially more time to generate
# 3 chars: seconds | 4 chars: minutes | 5 chars: hours | 6+ chars: days
VANITY_PREFIX=""

# Timeout before falling back to shorter prefix (seconds)
# If no match found in this time, prefix is shortened by 1 char
VANITY_TIMEOUT_SECONDS=30

#-------------------------------------------------------------------------------
# CAPTCHA SETTINGS
# CAPTCHA challenges protect against bots. Users solve one to get access.
#-------------------------------------------------------------------------------

# Enable CAPTCHA challenges (true/false)
CAPTCHA_ENABLED=true

# Target number of pre-generated CAPTCHAs to maintain
CAPTCHA_POOL_SIZE=500

# Minimum pool size before emergency generation kicks in
CAPTCHA_MIN_POOL=100

# Maximum pool size (hard cap to prevent memory issues)
CAPTCHA_MAX_POOL=1000

# Difficulty level (1-10, higher = harder)
# 1-3: Easy (most humans pass), 4-6: Medium, 7-10: Hard
CAPTCHA_DIFFICULTY=5

# Time limit to solve CAPTCHA (seconds)
CAPTCHA_TIMEOUT_SECONDS=120

# Maximum solve attempts before temporary ban
CAPTCHA_MAX_ATTEMPTS=3

# Enable audio CAPTCHA option (true/false)
CAPTCHA_AUDIO_ENABLED=false

# Pool rotation percentage (how much of the pool to replace)
CAPTCHA_ROTATION_PERCENT=25

# Pool rotation interval (days)
CAPTCHA_ROTATION_DAYS=10

#-------------------------------------------------------------------------------
# RATE LIMITING & THREAT DETECTION
# These thresholds control when Fortify takes action against suspicious traffic.
#-------------------------------------------------------------------------------

# Requests per minute before rate limiting kicks in
RATE_LIMIT_RPM=60

# Failed CAPTCHAs before temporary ban
CAPTCHA_FAIL_LIMIT=5

# Temporary ban duration (minutes)
TEMP_BAN_MINUTES=30

# Infractions before permanent ban
PERM_BAN_THRESHOLD=10

# Suspicion score threshold (0.0-1.0, lower = more sensitive)
SUSPICION_THRESHOLD=0.5

# Threat score threshold for immediate action (0.0-1.0)
THREAT_THRESHOLD=0.7

# Mirror burn threshold (0.0-1.0)
# When threat score exceeds this, mirror is burned
BURN_THRESHOLD=0.7

# Enable automatic banning (true/false)
AUTO_BAN_ENABLED=true

# DDoS detection: requests per second threshold
DDOS_RPS_THRESHOLD=100

# Probe detection sensitivity (1-10, higher = more sensitive)
PROBE_SENSITIVITY=5

#-------------------------------------------------------------------------------
# MIRROR LIFECYCLE SETTINGS
# Controls how mirrors are retired, burned, and potentially resurrected.
#-------------------------------------------------------------------------------

# Enable proactive (scheduled) burns (true/false)
# Proactive burns replace mirrors before they're attacked
PROACTIVE_BURN_ENABLED=true

# Minimum days between proactive burns
BURN_INTERVAL_DAYS_MIN=60

# Maximum days between proactive burns
BURN_INTERVAL_DAYS_MAX=120

# How long to show retirement page (hours)
# Users are redirected to new mirrors during this period
RETIREMENT_PAGE_HOURS=72

#-------------------------------------------------------------------------------
# RESURRECTION SETTINGS
# After a mirror is burned, Fortify can try to resurrect it if safe.
#-------------------------------------------------------------------------------

# Enable resurrection system (true/false)
RESURRECTION_ENABLED=true

# Wait time after burn before first evaluation (seconds)
RESURRECTION_WAIT_SECONDS=900

# Evaluation window duration (seconds)
RESURRECTION_EVAL_WINDOW=300

# Connection attempts above this = attack still ongoing
RESURRECTION_THREAT_THRESHOLD=50

# Connection attempts below this = safe to restore
RESURRECTION_SAFE_THRESHOLD=10

# Maximum days to keep dormant mirrors before permanent destruction
RESURRECTION_MAX_DORMANT_DAYS=90

#-------------------------------------------------------------------------------
# VANGUARDS SETTINGS
# Vanguards protect Tor circuits from guard discovery attacks.
#-------------------------------------------------------------------------------

# Enable vanguards addon (true/false)
# HIGHLY RECOMMENDED for production
VANGUARDS_ENABLED=true

# Layer 2 guards (middle relays)
VANGUARDS_LAYER2_GUARDS=4

# Layer 3 guards (entry guards)
VANGUARDS_LAYER3_GUARDS=8

#-------------------------------------------------------------------------------
# NODE POOL SETTINGS
# Nodes are internal workers that handle different types of traffic.
#-------------------------------------------------------------------------------

# Minimum healthy nodes (handle verified users)
MIN_HEALTHY_NODES=10

# Maximum healthy nodes
MAX_HEALTHY_NODES=20

# Minimum threat nodes (handle suspicious/attack traffic)
MIN_THREAT_NODES=3

# Maximum threat nodes
MAX_THREAT_NODES=10

# Number of orchestrators (mirror managers)
MIN_ORCHESTRATORS=2
MAX_ORCHESTRATORS=10

#-------------------------------------------------------------------------------
# AUTO-SCALING SETTINGS
# Auto-scaling adjusts resources based on load. USE WITH CAUTION.
#-------------------------------------------------------------------------------

# Enable auto-scaling (true/false)
# WARNING: Can consume significant resources during attacks
AUTO_SCALING_ENABLED=false

# Minimum standby pool size
AUTO_SCALING_MIN_STANDBY=1

# Maximum standby pool size
AUTO_SCALING_MAX_STANDBY=5

# Target standby pool size
AUTO_SCALING_TARGET_STANDBY=2

# Resource-aware mode (true/false)
# Checks CPU/memory before spawning
AUTO_SCALING_RESOURCE_AWARE=true

# Maximum CPU usage before refusing to spawn (percent)
AUTO_SCALING_MAX_CPU=80

# Maximum memory usage before refusing to spawn (percent)
AUTO_SCALING_MAX_MEMORY=85

# Minimum available memory before refusing to spawn (MB)
AUTO_SCALING_MIN_MEMORY_MB=512

# Self-DDOS protection: max spawns per minute
AUTO_SCALING_MAX_SPAWNS_PER_MIN=5

# Self-DDOS protection: max activations per minute
AUTO_SCALING_MAX_ACTIVATIONS_PER_MIN=10

#-------------------------------------------------------------------------------
# MULTI-DAEMON SETTINGS
# Multi-daemon mode runs one Tor instance per CPU core for maximum performance.
#-------------------------------------------------------------------------------

# Enable multi-daemon mode (true/false)
# Recommended for 4+ core systems
MULTI_DAEMON_ENABLED=false

# Number of daemons (0 = auto-detect from CPU cores)
MULTI_DAEMON_COUNT=0

# Enable CPU affinity pinning (true/false)
# Pins each Tor daemon to a specific CPU core
MULTI_DAEMON_CPU_AFFINITY=true

# Health check interval for daemons (seconds)
MULTI_DAEMON_HEALTH_INTERVAL=30

# Auto-restart failed daemons (true/false)
MULTI_DAEMON_AUTO_RESTART=true

#-------------------------------------------------------------------------------
# SELF-CLEANING SETTINGS
# Automatic cleanup of old data to prevent disk/memory exhaustion.
#-------------------------------------------------------------------------------

# Enable self-cleaning (true/false)
SELF_CLEANING_ENABLED=true

# Session cleanup interval (seconds)
CLEANUP_INTERVAL_SECONDS=300

# Session idle timeout (seconds before cleanup)
SESSION_IDLE_TIMEOUT_SECONDS=3600

# Maximum log file size (MB)
MAX_LOG_SIZE_MB=100

# Number of old log files to keep
LOG_RETENTION_COUNT=10

# Memory high-water mark (MB) - triggers cleanup
MEMORY_HIGH_WATER_MB=4096

# Remove burned mirrors' data after N days
BURNED_MIRROR_RETENTION_DAYS=7

# Remove destroyed mirrors' data after N days
DESTROYED_DATA_RETENTION_DAYS=30

#-------------------------------------------------------------------------------
# NETWORK SETTINGS
# Ports and addresses for internal communication.
#-------------------------------------------------------------------------------

# Tor SOCKS port (used for outbound connections)
TOR_SOCKS_PORT=9150

# Tor control port (used for circuit management)
TOR_CONTROL_PORT=9151

# HTTP proxy bind address (internal)
HTTP_BIND="127.0.0.1:8082"

# Gate bind address (internal)
GATE_BIND="127.0.0.1:8081"

# Controller bind address (internal)
CONTROLLER_BIND="127.0.0.1:7000"

# Orchestrator bind address (internal)
ORCHESTRATOR_BIND="127.0.0.1:8080"

#-------------------------------------------------------------------------------
# SECURITY SETTINGS
#-------------------------------------------------------------------------------

# Secret key for signing tokens (auto-generated if empty)
# IMPORTANT: Save this if you want sessions to survive restarts
SECRET_KEY=""

# Harden the OS (true/false)
# Applies sysctl settings, file limits, etc.
HARDEN_OS=true

#-------------------------------------------------------------------------------
# INSTALLATION SETTINGS
#-------------------------------------------------------------------------------

# Installation directory for binaries
INSTALL_DIR="/opt/fortify"

# Runtime data directory (Tor data, sessions, etc.)
DATA_DIR="/var/lib/fortify"

# Log directory
LOG_DIR="/var/log/fortify"

# Run Fortify as this user (created if doesn't exist)
FORTIFY_USER="fortify"

# Install as systemd services (true/false)
INSTALL_SYSTEMD=true

# Start services after installation (true/false)
START_AFTER_INSTALL=true

#===============================================================================
#                    END OF CONFIGURATION
#                    DO NOT EDIT BELOW THIS LINE
#===============================================================================

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Logging functions
log_info() { echo -e "${CYAN}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[✓]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[✗]${NC} $1"; }
log_step() { echo -e "\n${BLUE}[$1/$TOTAL_STEPS]${NC} $2"; }

TOTAL_STEPS=8
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_FILE="/tmp/fortify-deploy-$(date +%Y%m%d-%H%M%S).log"

# Start logging
exec > >(tee -a "$LOG_FILE") 2>&1

echo ""
echo "═══════════════════════════════════════════════════════════════════"
echo "                    FORTIFY DEPLOYMENT SCRIPT"
echo "═══════════════════════════════════════════════════════════════════"
echo ""

#-------------------------------------------------------------------------------
# PHASE 1: PREFLIGHT CHECKS
#-------------------------------------------------------------------------------
log_step 1 "PREFLIGHT CHECKS"

# Check if running as root
if [[ $EUID -ne 0 ]]; then
    log_error "This script must be run as root (use sudo)"
    exit 1
fi
log_success "Running as root"

# Detect OS
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS_NAME=$NAME
    OS_VERSION=$VERSION_ID
else
    log_error "Cannot detect OS. This script requires Ubuntu."
    exit 1
fi

if [[ ! "$OS_NAME" =~ "Ubuntu" ]]; then
    log_warn "This script is designed for Ubuntu. Detected: $OS_NAME"
    log_warn "Proceeding anyway, but some packages may fail to install."
fi
log_success "$OS_NAME $OS_VERSION detected"

# Check minimum requirements
CPU_CORES=$(nproc)
TOTAL_MEM=$(free -m | awk '/^Mem:/{print $2}')
AVAIL_DISK=$(df -m / | awk 'NR==2{print $4}')

log_info "System: ${CPU_CORES} CPU cores, ${TOTAL_MEM}MB RAM, ${AVAIL_DISK}MB disk available"

if [ "$TOTAL_MEM" -lt 1024 ]; then
    log_warn "Low memory detected (${TOTAL_MEM}MB). Recommended: 2048MB+"
fi

if [ "$AVAIL_DISK" -lt 1024 ]; then
    log_error "Insufficient disk space (${AVAIL_DISK}MB). Need at least 1024MB."
    exit 1
fi
log_success "System requirements met"

# Validate configuration
if [ -z "$BACKEND_ADDRESS" ]; then
    log_error "BACKEND_ADDRESS is required but not set"
    exit 1
fi
log_success "Configuration validated"

#-------------------------------------------------------------------------------
# DEPLOYMENT CONFLICT DETECTION & RESOLUTION
#-------------------------------------------------------------------------------
log_info "Checking for deployment conflicts..."

# Check 1: Old systemd services
CONFLICTS_FOUND=false
if systemctl is-active --quiet fortify 2>/dev/null || systemctl is-active --quiet fortifyd 2>/dev/null; then
    log_warn "Found active old systemd service(s)"
    systemctl is-active --quiet fortify 2>/dev/null && log_info "  - fortify.service (stopping...)"
    systemctl is-active --quiet fortifyd 2>/dev/null && log_info "  - fortifyd.service (stopping...)"
    
    systemctl stop fortify 2>/dev/null || true
    systemctl stop fortifyd 2>/dev/null || true
    systemctl disable fortify 2>/dev/null || true
    systemctl disable fortifyd 2>/dev/null || true
    
    log_success "Old services stopped and disabled"
    CONFLICTS_FOUND=true
fi

# Check 2: Port conflicts (8080-8090 range)
PORTS_IN_USE=$(netstat -tuln 2>/dev/null | grep -E ":(808[0-9]|8090)" | awk '{print $4}' | awk -F: '{print $NF}' | sort -u || true)
if [ -n "$PORTS_IN_USE" ]; then
    log_warn "Found processes using Fortify ports (8080-8090)"
    echo "$PORTS_IN_USE" | while read -r port; do
        PIDS=$(lsof -t -i :$port 2>/dev/null || true)
        if [ -n "$PIDS" ]; then
            log_info "  - Port $port (killing PIDs: $PIDS)"
            echo "$PIDS" | xargs -r kill -9 2>/dev/null || true
        fi
    done
    log_success "Port conflicts resolved"
    CONFLICTS_FOUND=true
fi

# Check 3: Existing Fortify processes
EXISTING_PROCS=$(pgrep -f 'fortify-|target/release/fortify' 2>/dev/null || true)
if [ -n "$EXISTING_PROCS" ]; then
    COUNT=$(echo "$EXISTING_PROCS" | wc -l)
    log_warn "Found $COUNT existing Fortify process(es)"
    pkill -9 -f 'fortify-' 2>/dev/null || true
    pkill -9 -f 'target/release/fortify' 2>/dev/null || true
    sleep 1
    log_success "Existing processes terminated"
    CONFLICTS_FOUND=true
fi

# Check 4: Stale PID files
if [ -d "/tmp/fortify" ]; then
    PID_FILES=$(find /tmp/fortify -name "*.pid" 2>/dev/null || true)
    if [ -n "$PID_FILES" ]; then
        rm -f /tmp/fortify/*.pid 2>/dev/null || true
        log_success "Cleaned up stale PID files"
        CONFLICTS_FOUND=true
    fi
fi

if [ "$CONFLICTS_FOUND" = false ]; then
    log_success "No deployment conflicts detected"
else
    log_success "All deployment conflicts resolved - ready to deploy"
fi

#-------------------------------------------------------------------------------
# PHASE 2: SYSTEM PREPARATION
#-------------------------------------------------------------------------------
log_step 2 "SYSTEM PREPARATION"

log_info "Updating package lists..."
apt-get update -qq

log_info "Upgrading existing packages..."
DEBIAN_FRONTEND=noninteractive apt-get upgrade -y -qq

log_info "Installing build dependencies..."
DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
    build-essential \
    pkg-config \
    libssl-dev \
    libsodium-dev \
    autoconf \
    automake \
    git \
    curl \
    wget \
    ca-certificates
log_success "Build dependencies installed"

log_info "Installing runtime dependencies..."
DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
    tor \
    python3 \
    python3-pip
log_success "Runtime dependencies installed"

# Install vanguards if enabled
if [ "$VANGUARDS_ENABLED" = true ]; then
    log_info "Installing vanguards..."
    pip3 install --break-system-packages --quiet vanguards 2>/dev/null || \
    pip3 install --quiet vanguards 2>/dev/null || \
    log_warn "Could not install vanguards via pip"
    log_success "Vanguards installed"
fi

# Install mkp224o if vanity is enabled
if [ "$VANITY_ENABLED" = true ]; then
    if ! command -v mkp224o &> /dev/null; then
        log_info "Building mkp224o from source (required for vanity addresses)..."
        
        MKP_BUILD_DIR=$(mktemp -d)
        cd "$MKP_BUILD_DIR"
        
        git clone --quiet https://github.com/cathugger/mkp224o.git
        cd mkp224o
        
        ./autogen.sh > /dev/null 2>&1
        ./configure --enable-donna > /dev/null 2>&1
        make -j"$CPU_CORES" > /dev/null 2>&1
        
        cp mkp224o /usr/local/bin/
        chmod +x /usr/local/bin/mkp224o
        
        cd /
        rm -rf "$MKP_BUILD_DIR"
        
        log_success "mkp224o installed"
    else
        log_success "mkp224o already installed"
    fi
fi

#-------------------------------------------------------------------------------
# PHASE 3: OS HARDENING
#-------------------------------------------------------------------------------
log_step 3 "OS HARDENING"

if [ "$HARDEN_OS" = true ]; then
    log_info "Applying sysctl hardening..."
    
    cat > /etc/sysctl.d/99-fortify.conf << 'EOF'
# Fortify OS Hardening

# Network hardening
net.ipv4.tcp_syncookies = 1
net.ipv4.tcp_max_syn_backlog = 65536
net.ipv4.tcp_synack_retries = 2
net.ipv4.tcp_syn_retries = 2
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.default.rp_filter = 1
net.ipv4.icmp_echo_ignore_broadcasts = 1
net.ipv4.icmp_ignore_bogus_error_responses = 1
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.default.accept_redirects = 0
net.ipv6.conf.all.accept_redirects = 0
net.ipv6.conf.default.accept_redirects = 0

# File descriptor limits
fs.file-max = 1000000
fs.nr_open = 1000000

# Memory settings
vm.swappiness = 10
vm.dirty_ratio = 60
vm.dirty_background_ratio = 2

# Connection tracking (for high connection counts)
net.netfilter.nf_conntrack_max = 1000000
net.netfilter.nf_conntrack_tcp_timeout_established = 86400
EOF

    sysctl -p /etc/sysctl.d/99-fortify.conf > /dev/null 2>&1 || true
    log_success "Sysctl hardening applied"

    log_info "Configuring file limits..."
    cat > /etc/security/limits.d/99-fortify.conf << EOF
# Fortify file limits
$FORTIFY_USER soft nofile 1000000
$FORTIFY_USER hard nofile 1000000
$FORTIFY_USER soft nproc 65535
$FORTIFY_USER hard nproc 65535
* soft nofile 1000000
* hard nofile 1000000
EOF
    log_success "File limits configured"
else
    log_info "OS hardening skipped (HARDEN_OS=false)"
fi

#-------------------------------------------------------------------------------
# PHASE 4: DOWNLOAD & BUILD
#-------------------------------------------------------------------------------
log_step 4 "DOWNLOAD & BUILD"

# Install Rust if not present
if ! command -v cargo &> /dev/null; then
    log_info "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --quiet
    source "$HOME/.cargo/env"
    log_success "Rust installed"
else
    log_success "Rust already installed"
fi

# Determine build directory
if [ -d "$SCRIPT_DIR/fortify/Cargo.toml" ] || [ -f "$SCRIPT_DIR/fortify/Cargo.toml" ]; then
    BUILD_DIR="$SCRIPT_DIR/fortify"
elif [ -f "$SCRIPT_DIR/Cargo.toml" ]; then
    BUILD_DIR="$SCRIPT_DIR"
else
    # Clone if running from elsewhere
    log_info "Cloning Fortify repository..."
    BUILD_DIR="/tmp/fortify-build"
    rm -rf "$BUILD_DIR"
    git clone --quiet https://github.com/Nespartious/Fortify.git "$BUILD_DIR"
    BUILD_DIR="$BUILD_DIR/fortify"
fi

log_info "Building Fortify (this may take a few minutes)..."
cd "$BUILD_DIR"
source "$HOME/.cargo/env" 2>/dev/null || true
cargo build --release --quiet 2>&1 | tail -5

log_success "Build complete"

#-------------------------------------------------------------------------------
# PHASE 5: GENERATE CONFIGURATION
#-------------------------------------------------------------------------------
log_step 5 "GENERATE CONFIGURATION"

# Create fortify user if it doesn't exist
if ! id "$FORTIFY_USER" &>/dev/null; then
    log_info "Creating fortify user..."
    useradd --system --no-create-home --shell /bin/false "$FORTIFY_USER"
    log_success "User '$FORTIFY_USER' created"
fi

# Generate secret key if not provided
if [ -z "$SECRET_KEY" ]; then
    SECRET_KEY=$(openssl rand -hex 32)
    log_info "Generated secret key (save this!): $SECRET_KEY"
fi

# Create directories
log_info "Creating directories..."
mkdir -p "$INSTALL_DIR/bin"
mkdir -p "$DATA_DIR/tor"
mkdir -p "$DATA_DIR/sessions"
mkdir -p "$DATA_DIR/captcha"
mkdir -p "$LOG_DIR"
mkdir -p /etc/fortify

# Copy binaries
log_info "Installing binaries..."
cp "$BUILD_DIR/target/release/fortify" "$INSTALL_DIR/bin/" 2>/dev/null || true
cp "$BUILD_DIR/target/release/fortify-controller" "$INSTALL_DIR/bin/"
cp "$BUILD_DIR/target/release/fortify-orchestrator" "$INSTALL_DIR/bin/"
cp "$BUILD_DIR/target/release/fortify-gate" "$INSTALL_DIR/bin/"
cp "$BUILD_DIR/target/release/fortify-http" "$INSTALL_DIR/bin/"
cp "$BUILD_DIR/target/release/fortify-node" "$INSTALL_DIR/bin/"
chmod +x "$INSTALL_DIR/bin/"*
log_success "Binaries installed to $INSTALL_DIR/bin/"

# Generate configuration file
log_info "Generating configuration..."
cat > /etc/fortify/fortify.toml << EOF
# Fortify Configuration
# Generated by deploy.sh on $(date)
# DO NOT EDIT - regenerate with deploy.sh

deployment_id = "$(openssl rand -hex 8)"

[branding]
service_name = "$SERVICE_NAME"
description = "$SERVICE_DESCRIPTION"
primary_color = "$PRIMARY_COLOR"
welcome_message = "$WELCOME_MESSAGE"
logo_max_width = 256
logo_max_height = 256

[captcha]
enabled = $CAPTCHA_ENABLED
pool_size = $CAPTCHA_POOL_SIZE
min_pool_size = $CAPTCHA_MIN_POOL
max_pool_size = $CAPTCHA_MAX_POOL
difficulty = $CAPTCHA_DIFFICULTY
timeout_seconds = $CAPTCHA_TIMEOUT_SECONDS
max_attempts = $CAPTCHA_MAX_ATTEMPTS
audio_enabled = $CAPTCHA_AUDIO_ENABLED
rotation_percent = $CAPTCHA_ROTATION_PERCENT
rotation_interval_days = $CAPTCHA_ROTATION_DAYS

[thresholds]
rate_limit_rpm = $RATE_LIMIT_RPM
captcha_fail_limit = $CAPTCHA_FAIL_LIMIT
temp_ban_minutes = $TEMP_BAN_MINUTES
perm_ban_threshold = $PERM_BAN_THRESHOLD
suspicion_threshold = $SUSPICION_THRESHOLD
threat_threshold = $THREAT_THRESHOLD
burn_threshold = $BURN_THRESHOLD
auto_ban_enabled = $AUTO_BAN_ENABLED
ddos_rps_threshold = $DDOS_RPS_THRESHOLD
probe_sensitivity = $PROBE_SENSITIVITY

[network]
backend_address = "$BACKEND_ADDRESS"
socks_port = $TOR_SOCKS_PORT
control_port = $TOR_CONTROL_PORT
http_bind = "$HTTP_BIND"
gate_bind = "$GATE_BIND"
vanguards_enabled = $VANGUARDS_ENABLED
vanguards_layer2 = $VANGUARDS_LAYER2_GUARDS
vanguards_layer3 = $VANGUARDS_LAYER3_GUARDS
data_dir = "$DATA_DIR"

[mirrors]
min_mirrors = $MIN_MIRRORS
max_mirrors = $MAX_MIRRORS
standby_mirrors = $STANDBY_MIRRORS
rotation_interval_seconds = $ROTATION_INTERVAL_SECONDS
proactive_burn_enabled = $PROACTIVE_BURN_ENABLED
burn_interval_days_min = $BURN_INTERVAL_DAYS_MIN
burn_interval_days_max = $BURN_INTERVAL_DAYS_MAX
retirement_page_hours = $RETIREMENT_PAGE_HOURS

[vanity]
enabled = $VANITY_ENABLED
prefix = "$VANITY_PREFIX"
safety_net_enabled = true
safety_net_timeout_seconds = $VANITY_TIMEOUT_SECONDS
min_prefix_length = 1
warn_threshold = 5
EOF
log_success "Configuration saved to /etc/fortify/fortify.toml"

# Set permissions
chown -R "$FORTIFY_USER:$FORTIFY_USER" "$DATA_DIR"
chown -R "$FORTIFY_USER:$FORTIFY_USER" "$LOG_DIR"
chmod 700 "$DATA_DIR"
chmod 755 "$LOG_DIR"

#-------------------------------------------------------------------------------
# PHASE 6: SYSTEMD SETUP
#-------------------------------------------------------------------------------
log_step 6 "SYSTEMD SETUP"

if [ "$INSTALL_SYSTEMD" = true ]; then
    log_info "Creating systemd service files..."

    # Environment file with all settings
    cat > /etc/fortify/environment << EOF
# Fortify Environment Variables
SECRET_KEY=$SECRET_KEY
FORTIFY_DATA_DIR=$DATA_DIR
BACKEND_ADDRESS=$BACKEND_ADDRESS
GATE_BIND_ADDR=$GATE_BIND
PROXY_BIND_ADDR=$HTTP_BIND
CONTROLLER_BIND_ADDR=$CONTROLLER_BIND
ORCH_BIND_ADDR=$ORCHESTRATOR_BIND
MIN_ORCHESTRATORS=$MIN_ORCHESTRATORS
MAX_ORCHESTRATORS=$MAX_ORCHESTRATORS
MIN_HEALTHY_NODES=$MIN_HEALTHY_NODES
MAX_HEALTHY_NODES=$MAX_HEALTHY_NODES
MIN_THREAT_NODES=$MIN_THREAT_NODES
MAX_THREAT_NODES=$MAX_THREAT_NODES
VANGUARDS_ENABLED=$VANGUARDS_ENABLED
VANGUARDS_LAYER2_GUARDS=$VANGUARDS_LAYER2_GUARDS
VANGUARDS_LAYER3_GUARDS=$VANGUARDS_LAYER3_GUARDS
VANITY_ENABLED=$VANITY_ENABLED
VANITY_PREFIX=$VANITY_PREFIX
VANITY_TIMEOUT=$VANITY_TIMEOUT_SECONDS
CAPTCHA_ENABLED=$CAPTCHA_ENABLED
CAPTCHA_POOL_SIZE=$CAPTCHA_POOL_SIZE
NODE_BACKEND_ADDR=$BACKEND_ADDRESS
EOF
    chmod 600 /etc/fortify/environment

    # Controller service
    cat > /etc/systemd/system/fortify-controller.service << EOF
[Unit]
Description=Fortify Controller - Resource & Process Manager
After=network.target tor.service
Wants=tor.service

[Service]
Type=simple
User=root
EnvironmentFile=/etc/fortify/environment
ExecStart=$INSTALL_DIR/bin/fortify-controller
Restart=always
RestartSec=5
LimitNOFILE=1000000
StandardOutput=append:$LOG_DIR/controller.log
StandardError=append:$LOG_DIR/controller.log

[Install]
WantedBy=multi-user.target
EOF

    # Orchestrator service
    cat > /etc/systemd/system/fortify-orchestrator.service << EOF
[Unit]
Description=Fortify Orchestrator - Mirror Management
After=network.target fortify-controller.service
Requires=fortify-controller.service

[Service]
Type=simple
User=$FORTIFY_USER
EnvironmentFile=/etc/fortify/environment
ExecStart=$INSTALL_DIR/bin/fortify-orchestrator
Restart=always
RestartSec=5
LimitNOFILE=1000000
StandardOutput=append:$LOG_DIR/orchestrator.log
StandardError=append:$LOG_DIR/orchestrator.log

[Install]
WantedBy=multi-user.target
EOF

    # Gate service
    cat > /etc/systemd/system/fortify-gate.service << EOF
[Unit]
Description=Fortify Gate - CAPTCHA Verification
After=network.target fortify-orchestrator.service
Requires=fortify-orchestrator.service

[Service]
Type=simple
User=$FORTIFY_USER
EnvironmentFile=/etc/fortify/environment
ExecStart=$INSTALL_DIR/bin/fortify-gate
Restart=always
RestartSec=5
LimitNOFILE=1000000
StandardOutput=append:$LOG_DIR/gate.log
StandardError=append:$LOG_DIR/gate.log

[Install]
WantedBy=multi-user.target
EOF

    # HTTP Proxy service
    cat > /etc/systemd/system/fortify-http.service << EOF
[Unit]
Description=Fortify HTTP Proxy - Traffic Handler
After=network.target fortify-gate.service
Requires=fortify-gate.service

[Service]
Type=simple
User=$FORTIFY_USER
EnvironmentFile=/etc/fortify/environment
ExecStart=$INSTALL_DIR/bin/fortify-http
Restart=always
RestartSec=5
LimitNOFILE=1000000
StandardOutput=append:$LOG_DIR/http.log
StandardError=append:$LOG_DIR/http.log

[Install]
WantedBy=multi-user.target
EOF

    # Reload systemd
    systemctl daemon-reload
    
    # Enable services
    systemctl enable fortify-controller fortify-orchestrator fortify-gate fortify-http
    
    log_success "Systemd services installed and enabled"
else
    log_info "Systemd setup skipped (INSTALL_SYSTEMD=false)"
fi

#-------------------------------------------------------------------------------
# PHASE 7: DEPLOY & VERIFY
#-------------------------------------------------------------------------------
log_step 7 "DEPLOY & VERIFY"

if [ "$START_AFTER_INSTALL" = true ] && [ "$INSTALL_SYSTEMD" = true ]; then
    log_info "Starting Fortify services..."
    
    systemctl start fortify-controller
    sleep 2
    
    systemctl start fortify-orchestrator
    sleep 2
    
    systemctl start fortify-gate
    sleep 1
    
    systemctl start fortify-http
    sleep 2
    
    # Verify services are running
    FAILED=0
    
    if systemctl is-active --quiet fortify-controller; then
        log_success "Controller: running"
    else
        log_error "Controller: failed"
        FAILED=1
    fi
    
    if systemctl is-active --quiet fortify-orchestrator; then
        log_success "Orchestrator: running"
    else
        log_error "Orchestrator: failed"
        FAILED=1
    fi
    
    if systemctl is-active --quiet fortify-gate; then
        log_success "Gate: running"
    else
        log_error "Gate: failed"
        FAILED=1
    fi
    
    if systemctl is-active --quiet fortify-http; then
        log_success "HTTP Proxy: running"
    else
        log_error "HTTP Proxy: failed"
        FAILED=1
    fi
    
    if [ $FAILED -eq 1 ]; then
        log_error "Some services failed to start. Check logs in $LOG_DIR/"
    fi
else
    log_info "Service startup skipped"
fi

#-------------------------------------------------------------------------------
# PHASE 8: COMPLETE
#-------------------------------------------------------------------------------
log_step 8 "COMPLETE"

echo ""
echo "═══════════════════════════════════════════════════════════════════"
echo ""
echo -e "${GREEN}🏰 FORTIFY DEPLOYMENT COMPLETE${NC}"
echo ""
echo "═══════════════════════════════════════════════════════════════════"
echo ""
echo "Protected Backend: $BACKEND_ADDRESS"
echo ""
echo "Configuration:     /etc/fortify/fortify.toml"
echo "Data Directory:    $DATA_DIR"
echo "Log Directory:     $LOG_DIR"
echo "Binaries:          $INSTALL_DIR/bin/"
echo ""
echo "Secret Key (SAVE THIS!):"
echo "  $SECRET_KEY"
echo ""
echo "Management Commands:"
echo "  systemctl status fortify-*           # Check status"
echo "  systemctl restart fortify-controller # Restart all"
echo "  journalctl -u fortify-http -f        # View HTTP logs"
echo "  $INSTALL_DIR/bin/fortify             # Launch TUI"
echo ""
echo "To view .onion addresses (after Tor circuits establish):"
echo "  ls $DATA_DIR/tor/*/hostname"
echo ""
echo "Deployment log: $LOG_FILE"
echo ""
echo "═══════════════════════════════════════════════════════════════════"
echo ""
