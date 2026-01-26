#!/bin/bash
# Pre-deployment conflict detection and resolution
# 
# NOTE: This script is now AUTOMATICALLY RUN by:
#   - deploy.sh (headless deployments)
#   - start-fortify.sh (manual starts)
#
# You generally DON'T need to run this manually anymore.
# It's kept as a standalone tool for debugging deployment issues.
#
# Automatically detects and resolves port conflicts and old service instances

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FORTIFY_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }

echo -e "\n${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  Fortify Pre-Deployment Conflict Check ${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}\n"

# Check 1: Old systemd service
log_info "Checking for old systemd services..."
if systemctl is-active --quiet fortify 2>/dev/null || systemctl is-active --quiet fortifyd 2>/dev/null; then
    log_warn "Found active systemd service(s)"
    
    if systemctl is-active --quiet fortify; then
        echo "  - fortify.service is running"
        systemctl status fortify --no-pager | grep -E "(Active|Main PID|Memory)" || true
    fi
    
    if systemctl is-active --quiet fortifyd; then
        echo "  - fortifyd.service is running"
        systemctl status fortifyd --no-pager | grep -E "(Active|Main PID|Memory)" || true
    fi
    
    echo ""
    read -p "Stop these services before deploying? [Y/n] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]] || [[ -z $REPLY ]]; then
        log_info "Stopping old services..."
        sudo systemctl stop fortify 2>/dev/null || true
        sudo systemctl stop fortifyd 2>/dev/null || true
        log_success "Services stopped"
    else
        log_error "Cannot deploy with conflicting services running"
        exit 1
    fi
else
    log_success "No conflicting systemd services found"
fi

# Check 2: Port conflicts (8080-8090 range)
log_info "Checking for port conflicts..."
PORTS_IN_USE=$(netstat -tuln 2>/dev/null | grep -E ":(808[0-9]|8090)" | awk '{print $4}' | awk -F: '{print $NF}' | sort -u)

if [ -n "$PORTS_IN_USE" ]; then
    log_warn "Found processes using Fortify ports:"
    echo "$PORTS_IN_USE" | while read -r port; do
        echo "  Port $port:"
        sudo lsof -i :$port -n -P 2>/dev/null | grep LISTEN || true
    done
    
    echo ""
    read -p "Kill these processes? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        log_info "Terminating processes..."
        echo "$PORTS_IN_USE" | while read -r port; do
            sudo lsof -t -i :$port 2>/dev/null | xargs -r sudo kill -9 2>/dev/null || true
        done
        log_success "Processes terminated"
    else
        log_warn "Proceeding with port conflicts - deployment may fail"
    fi
else
    log_success "No port conflicts detected"
fi

# Check 3: Existing Fortify processes
log_info "Checking for existing Fortify processes..."
EXISTING_PROCS=$(ps aux | grep -E 'fortify-|target/release/fortify' | grep -v grep | grep -v pre-deploy-check || true)

if [ -n "$EXISTING_PROCS" ]; then
    log_warn "Found existing Fortify processes:"
    echo "$EXISTING_PROCS" | head -5
    
    COUNT=$(echo "$EXISTING_PROCS" | wc -l)
    if [ "$COUNT" -gt 5 ]; then
        echo "  ... and $((COUNT - 5)) more"
    fi
    
    echo ""
    read -p "Kill these processes? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        log_info "Terminating Fortify processes..."
        pkill -9 -f 'fortify-' 2>/dev/null || true
        pkill -9 -f 'target/release/fortify' 2>/dev/null || true
        log_success "Processes terminated"
    else
        log_warn "Existing processes will conflict with new deployment"
    fi
else
    log_success "No existing Fortify processes found"
fi

# Check 4: PID files
log_info "Checking for stale PID files..."
if [ -d "/tmp/fortify" ]; then
    PID_FILES=$(find /tmp/fortify -name "*.pid" 2>/dev/null)
    if [ -n "$PID_FILES" ]; then
        log_warn "Found PID files:"
        echo "$PID_FILES"
        rm -f /tmp/fortify/*.pid 2>/dev/null || true
        log_success "Cleaned up PID files"
    else
        log_success "No stale PID files"
    fi
fi

# Check 5: Build artifacts
log_info "Checking build status..."
if [ ! -f "$FORTIFY_ROOT/target/release/fortify" ]; then
    log_error "Release binary not found. Run: cargo build --release"
    exit 1
else
    BUILD_AGE=$(find "$FORTIFY_ROOT/target/release/fortify" -mmin +60 2>/dev/null)
    if [ -n "$BUILD_AGE" ]; then
        log_warn "Release binary is older than 1 hour"
        echo "  Consider rebuilding with: cargo build --release"
    else
        log_success "Recent release build found"
    fi
fi

echo ""
log_success "Pre-deployment checks complete - ready to deploy!"
echo ""
