#!/bin/bash
#
# Fortify Deployment Script - ENTERPRISE Tier
# ~1,000,000+ users/day - High-traffic platforms
#
# For high-traffic platforms requiring maximum capacity.
# Requires high-end dedicated server(s) or cluster.
#
# System Requirements:
#   CPU: 16+ cores
#   RAM: 32GB+
#   Disk: 100GB+ SSD/NVMe
#   Network: 1Gbps+
#
# ============================================================================

set -e

# Traffic Tier: ENTERPRISE (~1,000,000+ users/day)
export FORTIFY_TRAFFIC_TIER="enterprise"

# ============================================================================
# CAPTCHA POOL SETTINGS (Enterprise: Maximum)
# ============================================================================
POOL_SIZE=10000          # Maximum pool size
MIN_POOL_SIZE=2000       # 20% emergency threshold
MAX_POOL_SIZE=20000      # 2x target for extreme bursts

# ============================================================================
# RATE LIMITING (Enterprise: Maximum Permissive)
# ============================================================================
RATE_LIMIT_RPM=600       # 600 requests per minute per circuit (10 RPS)
DDOS_RPS_THRESHOLD=10000 # Very high DDoS threshold

# ============================================================================
# MIRROR SETTINGS (Enterprise: Maximum Scale)
# ============================================================================
MIN_MIRRORS=10           # 10 active mirrors minimum
MAX_MIRRORS=50           # Scale up to 50 mirrors
STANDBY_MIRRORS=10       # 10 standby mirrors

# ============================================================================
# BAN THRESHOLDS (Enterprise: Very Lenient)
# ============================================================================
TEMP_BAN_MINUTES=5       # Very short bans
PERM_BAN_THRESHOLD=30    # Many violations before perm ban

# ============================================================================
# CAPTCHA SETTINGS
# ============================================================================
CAPTCHA_DIFFICULTY=5
CAPTCHA_TIMEOUT_SECONDS=120
CAPTCHA_MAX_ATTEMPTS=3

# ============================================================================
# BACKEND SETTINGS (Configure These)
# ============================================================================
BACKEND_ADDRESS="http://127.0.0.1:9000"
SERVICE_NAME="Protected Service"
SERVICE_DESCRIPTION="A Fortify-protected onion service"
PRIMARY_COLOR="#c9a227"
SECONDARY_COLOR="#a68b5b"

# ============================================================================
# NETWORK SETTINGS
# ============================================================================
SOCKS_PORT=9150
CONTROL_PORT=9151
HTTP_BIND="127.0.0.1:8082"
GATE_BIND="127.0.0.1:8081"

# ============================================================================
# DEPLOYMENT
# ============================================================================
echo "========================================"
echo "Fortify Deployment - ENTERPRISE Tier"
echo "Expected traffic: ~1,000,000+ users/day"
echo "========================================"
echo ""
echo "Configuration:"
echo "  CAPTCHA Pool: $POOL_SIZE (min: $MIN_POOL_SIZE, max: $MAX_POOL_SIZE)"
echo "  Rate Limit: $RATE_LIMIT_RPM RPM"
echo "  DDoS Threshold: $DDOS_RPS_THRESHOLD RPS"
echo "  Mirrors: $MIN_MIRRORS-$MAX_MIRRORS active, $STANDBY_MIRRORS standby"
echo ""
echo "WARNING: Enterprise tier requires significant resources!"
echo "         Make sure your server meets the requirements."
echo ""

# Check if main deploy.sh exists and source common deployment logic
if [ -f "../deploy.sh" ]; then
    echo "Running main deployment script with ENTERPRISE settings..."
    export POOL_SIZE MIN_POOL_SIZE MAX_POOL_SIZE
    export RATE_LIMIT_RPM DDOS_RPS_THRESHOLD
    export MIN_MIRRORS MAX_MIRRORS STANDBY_MIRRORS
    export TEMP_BAN_MINUTES PERM_BAN_THRESHOLD
    export CAPTCHA_DIFFICULTY CAPTCHA_TIMEOUT_SECONDS CAPTCHA_MAX_ATTEMPTS
    export BACKEND_ADDRESS SERVICE_NAME SERVICE_DESCRIPTION
    export PRIMARY_COLOR SECONDARY_COLOR
    export SOCKS_PORT CONTROL_PORT HTTP_BIND GATE_BIND
    
    cd .. && ./deploy.sh
else
    echo "Error: Main deploy.sh not found in parent directory"
    echo "Please run this script from the Deploy-Scripts directory"
    exit 1
fi
