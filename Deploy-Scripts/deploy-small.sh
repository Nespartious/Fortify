#!/bin/bash
#
# Fortify Deployment Script - SMALL Tier (DEFAULT)
# ~1,000 users/day - Small community sites
#
# This is the default configuration, balanced for small
# community sites with moderate traffic.
#
# System Requirements:
#   CPU: 2 cores (4 recommended)
#   RAM: 1-2GB
#   Disk: 1GB
#
# ============================================================================

set -e

# Traffic Tier: SMALL (~1,000 users/day) - DEFAULT
export FORTIFY_TRAFFIC_TIER="small"

# ============================================================================
# CAPTCHA POOL SETTINGS (Small: Balanced)
# ============================================================================
POOL_SIZE=500          # Standard pool size
MIN_POOL_SIZE=100      # Emergency threshold at 20%
MAX_POOL_SIZE=1000     # 2x target for burst capacity

# ============================================================================
# RATE LIMITING (Small: Balanced)
# ============================================================================
RATE_LIMIT_RPM=60      # 60 requests per minute per circuit (1 RPS)
DDOS_RPS_THRESHOLD=100 # Detect DDoS at 100 requests/second

# ============================================================================
# MIRROR SETTINGS (Small: Standard)
# ============================================================================
MIN_MIRRORS=2          # 2 active mirrors for redundancy
MAX_MIRRORS=5          # Scale up to 5 if needed
STANDBY_MIRRORS=2      # 2 standby mirrors

# ============================================================================
# BAN THRESHOLDS (Small: Balanced)
# ============================================================================
TEMP_BAN_MINUTES=30    # Standard temp ban duration
PERM_BAN_THRESHOLD=10  # Standard permanent ban threshold

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
echo "Fortify Deployment - SMALL Tier (DEFAULT)"
echo "Expected traffic: ~1,000 users/day"
echo "========================================"
echo ""
echo "Configuration:"
echo "  CAPTCHA Pool: $POOL_SIZE (min: $MIN_POOL_SIZE, max: $MAX_POOL_SIZE)"
echo "  Rate Limit: $RATE_LIMIT_RPM RPM"
echo "  DDoS Threshold: $DDOS_RPS_THRESHOLD RPS"
echo "  Mirrors: $MIN_MIRRORS-$MAX_MIRRORS active, $STANDBY_MIRRORS standby"
echo ""

# Check if main deploy.sh exists and source common deployment logic
if [ -f "../deploy.sh" ]; then
    echo "Running main deployment script with SMALL settings..."
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
