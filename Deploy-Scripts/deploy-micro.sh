#!/bin/bash
#
# Fortify Deployment Script - MICRO Tier
# ~100 users/day - Personal or test deployments
#
# This script configures Fortify for minimal resource usage,
# suitable for testing or very low-traffic personal sites.
#
# System Requirements:
#   CPU: 1-2 cores (even ARM)
#   RAM: 512MB-1GB
#   Disk: 500MB
#
# ============================================================================

set -e

# Traffic Tier: MICRO (~100 users/day)
export FORTIFY_TRAFFIC_TIER="micro"

# ============================================================================
# CAPTCHA POOL SETTINGS (Micro: Conservative)
# ============================================================================
POOL_SIZE=50           # Small pool - low traffic
MIN_POOL_SIZE=10       # Minimal emergency threshold
MAX_POOL_SIZE=100      # Cap for memory conservation

# ============================================================================
# RATE LIMITING (Micro: Strict)
# ============================================================================
RATE_LIMIT_RPM=30      # 30 requests per minute per circuit
DDOS_RPS_THRESHOLD=20  # Detect DDoS at 20 requests/second

# ============================================================================
# MIRROR SETTINGS (Micro: Minimal)
# ============================================================================
MIN_MIRRORS=1          # Single active mirror
MAX_MIRRORS=2          # Maximum 2 mirrors
STANDBY_MIRRORS=1      # 1 standby mirror

# ============================================================================
# BAN THRESHOLDS (Micro: Aggressive)
# ============================================================================
TEMP_BAN_MINUTES=60    # Longer temp bans (low volume, can be strict)
PERM_BAN_THRESHOLD=5   # Fewer infractions before permanent ban

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
echo "Fortify Deployment - MICRO Tier"
echo "Expected traffic: ~100 users/day"
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
    echo "Running main deployment script with MICRO settings..."
    # Export all variables for the main script
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
