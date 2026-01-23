#!/bin/bash
#
# Fortify Deployment Script - LARGE Tier
# ~100,000 users/day - Popular services
#
# For popular services with heavy traffic.
# Requires powerful dedicated server.
#
# System Requirements:
#   CPU: 4 cores minimum, 8 cores recommended
#   RAM: 4GB minimum, 8GB recommended
#   Disk: 5GB SSD minimum
#
# ============================================================================

set -e

# Traffic Tier: LARGE (~100,000 users/day)
export FORTIFY_TRAFFIC_TIER="large"

# ============================================================================
# CAPTCHA POOL SETTINGS (Large: High Capacity)
# ============================================================================
POOL_SIZE=3000          # Reduced from 5K for Tor realism
MIN_POOL_SIZE=1000      # 33% emergency threshold
MAX_POOL_SIZE=6000      # 2x target for bursts

# ============================================================================
# RATE LIMITING (Large: Very Permissive)
# ============================================================================
RATE_LIMIT_RPM=300      # 300 requests per minute per circuit (5 RPS)
DDOS_RPS_THRESHOLD=1000 # Reduced from 2K for Tor realism

# ============================================================================
# MIRROR SETTINGS (Large: Robust)
# ============================================================================
MIN_MIRRORS=4           # 4 active mirrors minimum
MAX_MIRRORS=12          # Reduced from 20 for Tor realism
STANDBY_MIRRORS=4       # 4 standby mirrors

# ============================================================================
# BAN THRESHOLDS (Large: Lenient)
# ============================================================================
TEMP_BAN_MINUTES=10     # Short bans (high volume)
PERM_BAN_THRESHOLD=20   # More violations before perm ban

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
echo "Fortify Deployment - LARGE Tier"
echo "Expected traffic: ~100,000 users/day"
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
    echo "Running main deployment script with LARGE settings..."
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
