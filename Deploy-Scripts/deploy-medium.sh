#!/bin/bash
#
# Fortify Deployment Script - MEDIUM Tier
# ~10,000 users/day - Active community sites
#
# For active community sites with consistent traffic.
# Requires a dedicated VPS or server.
#
# System Requirements:
#   CPU: 4 cores
#   RAM: 4GB
#   Disk: 5GB
#
# ============================================================================

set -e

# Traffic Tier: MEDIUM (~10,000 users/day)
export FORTIFY_TRAFFIC_TIER="medium"

# ============================================================================
# CAPTCHA POOL SETTINGS (Medium: Generous)
# ============================================================================
POOL_SIZE=2000         # Larger pool for higher traffic
MIN_POOL_SIZE=500      # 25% emergency threshold
MAX_POOL_SIZE=5000     # 2.5x target for bursts

# ============================================================================
# RATE LIMITING (Medium: Permissive)
# ============================================================================
RATE_LIMIT_RPM=120     # 120 requests per minute per circuit (2 RPS)
DDOS_RPS_THRESHOLD=500 # Higher DDoS threshold for busy sites

# ============================================================================
# MIRROR SETTINGS (Medium: Robust)
# ============================================================================
MIN_MIRRORS=3          # 3 active mirrors
MAX_MIRRORS=10         # Scale up to 10
STANDBY_MIRRORS=3      # 3 standby mirrors

# ============================================================================
# BAN THRESHOLDS (Medium: Moderate)
# ============================================================================
TEMP_BAN_MINUTES=15    # Shorter bans (high volume, more false positives)
PERM_BAN_THRESHOLD=15  # More lenient permanent ban threshold

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
echo "Fortify Deployment - MEDIUM Tier"
echo "Expected traffic: ~10,000 users/day"
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
    echo "Running main deployment script with MEDIUM settings..."
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
