#!/bin/bash
# Test Attack Mode - Trigger and verify Fortify's attack detection
#
# Usage:
#   ./test-attack-mode.sh trigger   - Send rapid requests to trigger attack mode
#   ./test-attack-mode.sh status    - Check current system status
#   ./test-attack-mode.sh clear     - Wait for attack mode to clear
#
# Attack mode is triggered by high request rates and clears automatically after ~2 minutes

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
NC='\033[0m'

GATE_PORT="${GATE_PORT:-8081}"
CONTROLLER_PORT="${CONTROLLER_PORT:-8080}"

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[✓]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[⚠]${NC} $1"; }
log_attack() { echo -e "${RED}[🔴 ATTACK]${NC} $1"; }

show_usage() {
    echo "Fortify Attack Mode Tester"
    echo ""
    echo "Usage: $0 <command>"
    echo ""
    echo "Commands:"
    echo "  trigger   Send 200 requests via Tor to trigger attack mode"
    echo "  blast     Send 500 requests at max speed via Tor (aggressive)"
    echo "  local     Send 200 rapid requests directly to gate (no Tor, faster)"
    echo "  status    Check controller health and mirror list"
    echo "  pool      Check prerendered page pool status"
    echo "  clear     Wait for attack mode to naturally clear (~2 min)"
    echo ""
    echo "Example workflow:"
    echo "  1. $0 status   # Check current state"
    echo "  2. $0 local    # Quick local test (no Tor needed)"
    echo "  3. $0 trigger  # Full test via Tor"
    echo "  4. $0 pool     # Check pool stats during attack"
    echo "  5. $0 clear    # Wait for it to clear"
}

check_status() {
    log_info "Checking Fortify status..."
    echo ""
    
    # Check controller health
    echo -e "${MAGENTA}=== Controller Health (port $CONTROLLER_PORT) ===${NC}"
    local health=$(curl -s "http://127.0.0.1:$CONTROLLER_PORT/health" 2>/dev/null)
    if [ -n "$health" ]; then
        echo "$health" | tr ',' '\n' | tr '{' ' ' | tr '}' ' ' | sed 's/"//g'
    else
        echo -e "${RED}Controller not responding${NC}"
    fi
    echo ""
    
    # Check mirrors
    echo -e "${MAGENTA}=== Mirrors ===${NC}"
    local mirrors=$(curl -s "http://127.0.0.1:$CONTROLLER_PORT/mirrors" 2>/dev/null)
    if [ -n "$mirrors" ]; then
        # Extract mirror addresses
        echo "$mirrors" | grep -oP '[a-z0-9]{56}\.onion' | while read -r addr; do
            echo "  • $addr"
        done
        local count=$(echo "$mirrors" | grep -oP '"count":\d+' | grep -oP '\d+')
        echo "  Total: ${count:-0} mirrors"
    else
        echo "  No mirrors found"
    fi
    echo ""
    
    # Try to hit a mirror directly
    echo -e "${MAGENTA}=== Mirror Connectivity Test ===${NC}"
    local first_mirror=$(echo "$mirrors" | grep -oP '[a-z0-9]{56}\.onion' | head -1)
    if [ -n "$first_mirror" ]; then
        log_info "Testing first mirror via Tor..."
        local status=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
            --socks5-hostname 127.0.0.1:9150 "http://$first_mirror/" 2>/dev/null || echo "timeout")
        echo "  Status: $status"
    fi
    echo ""
}

check_pool() {
    log_info "Checking prerendered page pool..."
    echo ""
    
    echo -e "${MAGENTA}=== Pool Statistics ===${NC}"
    local stats=$(curl -s "http://127.0.0.1:$GATE_PORT/gate/api/pool-stats" 2>/dev/null)
    if [ -n "$stats" ] && [ "$stats" != "" ]; then
        # Pretty print without jq
        echo "$stats" | tr ',' '\n' | tr '{' '\n' | tr '}' '\n' | grep -E ':' | sed 's/"//g' | sed 's/^/  /'
    else
        echo -e "${YELLOW}Pool stats not available (gate may not be running locally)${NC}"
        echo "  Try checking via mirror:"
        echo "  curl http://<mirror-address>/gate/api/pool-stats"
    fi
    echo ""
}

trigger_attack() {
    local count="${1:-200}"
    local delay="${2:-0.01}"
    
    log_attack "Triggering attack mode with $count rapid requests..."
    echo ""
    
    # Get a mirror address from /mirrors endpoint
    local mirrors=$(curl -s "http://127.0.0.1:$CONTROLLER_PORT/mirrors" 2>/dev/null)
    local mirror=""
    
    if [ -n "$mirrors" ]; then
        # Extract first onion address
        mirror=$(echo "$mirrors" | grep -oP '[a-z0-9]{56}\.onion' | head -1)
    fi
    
    local use_tor=false
    local target=""
    
    if [ -n "$mirror" ]; then
        log_info "Found mirror: $mirror"
        log_info "Using Tor SOCKS proxy on port 9150..."
        use_tor=true
        target="$mirror"
    else
        log_warn "No mirror found, using gate directly on port $GATE_PORT"
        target="127.0.0.1:$GATE_PORT"
    fi
    
    log_info "Target: http://$target/"
    echo ""
    
    local success=0
    local blocked=0
    local errors=0
    
    echo -n "Sending $count requests: "
    for i in $(seq 1 $count); do
        # Send request and capture status
        local status
        if [ "$use_tor" = true ]; then
            status=$(curl -s -o /dev/null -w "%{http_code}" --max-time 5 \
                --socks5-hostname 127.0.0.1:9150 "http://$target/" 2>/dev/null) || status="000"
        else
            status=$(curl -s -o /dev/null -w "%{http_code}" --max-time 2 \
                "http://$target/" 2>/dev/null) || status="000"
        fi
        
        case "$status" in
            200|302|303) success=$((success + 1)) ;;
            429|503) blocked=$((blocked + 1)) ;;
            *) errors=$((errors + 1)) ;;
        esac
        
        # Progress indicator every 10 requests
        if [ $((i % 10)) -eq 0 ]; then
            echo -n "."
        fi
        
        # Small delay to control rate (skip if delay is 0)
        if [ "$delay" != "0" ]; then
            sleep "$delay" 2>/dev/null || true
        fi
    done
    
    echo " done!"
    echo ""
    log_info "Results:"
    echo "  Success (200/302/303): $success"
    echo "  Blocked (429/503): $blocked"  
    echo "  Errors/Timeout: $errors"
    echo ""
    
    if [ $blocked -gt 0 ]; then
        log_attack "Attack mode likely ACTIVE - $blocked requests blocked"
    elif [ $errors -gt $((count / 2)) ]; then
        log_warn "Many errors - Tor may be slow or target unreachable"
    else
        log_success "All requests served - system handling load well"
    fi
}

blast_attack() {
    log_attack "BLAST MODE - Sending 500 requests at maximum speed..."
    trigger_attack 500 0
}

local_attack() {
    local count="${1:-200}"
    
    log_attack "LOCAL MODE - Sending $count requests directly to gate (no Tor)..."
    echo ""
    
    local target="127.0.0.1:$GATE_PORT"
    log_info "Target: http://$target/"
    echo ""
    
    local success=0
    local blocked=0
    local errors=0
    
    echo -n "Sending $count requests: "
    for i in $(seq 1 $count); do
        local status
        status=$(curl -s -o /dev/null -w "%{http_code}" --max-time 2 "http://$target/" 2>/dev/null) || status="000"
        
        case "$status" in
            200|302|303) success=$((success + 1)) ;;
            429|503) blocked=$((blocked + 1)) ;;
            *) errors=$((errors + 1)) ;;
        esac
        
        # Progress every 10 requests
        if [ $((i % 10)) -eq 0 ]; then
            echo -n "."
        fi
    done
    
    echo " done!"
    echo ""
    log_info "Results:"
    echo "  Success (200/302/303): $success"
    echo "  Blocked (429/503): $blocked"  
    echo "  Errors/Timeout: $errors"
    echo ""
    
    if [ $blocked -gt 0 ]; then
        log_attack "Attack mode likely ACTIVE - $blocked requests blocked"
    elif [ $errors -gt 50 ]; then
        log_warn "Many errors - is gate running on port $GATE_PORT?"
    else
        log_success "All requests served - system handling load well"
    fi
}

wait_clear() {
    log_info "Waiting for attack mode to clear (typically ~2 minutes)..."
    echo ""
    
    local start=$(date +%s)
    local max_wait=180  # 3 minutes max
    
    while true; do
        local elapsed=$(($(date +%s) - start))
        
        if [ $elapsed -ge $max_wait ]; then
            log_warn "Timeout - attack mode may still be active"
            break
        fi
        
        # Check if requests are being accepted
        local status=$(curl -s -o /dev/null -w "%{http_code}" --max-time 2 "http://127.0.0.1:$GATE_PORT/" 2>/dev/null || echo "000")
        
        if [ "$status" = "200" ] || [ "$status" = "302" ]; then
            # Send a few more to confirm
            sleep 1
            status=$(curl -s -o /dev/null -w "%{http_code}" --max-time 2 "http://127.0.0.1:$GATE_PORT/" 2>/dev/null || echo "000")
            if [ "$status" = "200" ] || [ "$status" = "302" ]; then
                log_success "Attack mode cleared after ${elapsed}s"
                break
            fi
        fi
        
        echo -ne "\r  Elapsed: ${elapsed}s / ${max_wait}s (status: $status)"
        sleep 5
    done
    echo ""
}

# Main
case "${1:-}" in
    trigger)
        trigger_attack 200 0.01
        ;;
    blast)
        blast_attack
        ;;
    local)
        local_attack 200
        ;;
    status)
        check_status
        ;;
    pool)
        check_pool
        ;;
    clear)
        wait_clear
        ;;
    *)
        show_usage
        exit 1
        ;;
esac
