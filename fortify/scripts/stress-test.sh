#!/bin/bash
# Automated Attack Stress Test
#
# This script runs automated attacks against Fortify without requiring
# manual CAPTCHA solving. It tests the system's resilience under load.
#
# Usage: ./stress-test.sh [mirror_address] [duration_seconds]

set -e

MIRROR="${1:-}"
DURATION="${2:-60}"
TOR_SOCKS_PORT="${TOR_SOCKS_PORT:-9150}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[✓]${NC} $1"; }
log_error() { echo -e "${RED}[✗]${NC} $1"; }

# Get mirror address
if [ -z "$MIRROR" ]; then
    log_info "Fetching mirror address from controller..."
    MIRROR=$(curl -s http://127.0.0.1:8080/mirrors | jq -r '.mirrors[0].onion_address' 2>/dev/null)
    
    if [ -z "$MIRROR" ] || [ "$MIRROR" = "null" ]; then
        log_error "No mirrors found. Is Fortify running?"
        exit 1
    fi
fi

log_info "Target Mirror: $MIRROR"
log_info "Test Duration: ${DURATION}s"
echo ""

# Metrics tracking
TOTAL_REQUESTS=0
BLOCKED_REQUESTS=0
SUCCESS_REQUESTS=0
TIMEOUT_REQUESTS=0

# Attack 1: Rapid fire from single circuit
attack_single_circuit() {
    local count=$1
    log_info "Attack 1: Rapid fire from single circuit ($count requests)..."
    
    for i in $(seq 1 $count); do
        local status=$(curl -s --max-time 5 --socks5-hostname 127.0.0.1:$TOR_SOCKS_PORT \
            -o /dev/null -w "%{http_code}" \
            "http://$MIRROR/" 2>/dev/null || echo "000")
        
        ((TOTAL_REQUESTS++))
        
        case "$status" in
            200) ((SUCCESS_REQUESTS++)); echo -n "." ;;
            429) ((BLOCKED_REQUESTS++)); echo -n "R" ;;
            403) ((BLOCKED_REQUESTS++)); echo -n "B" ;;
            000) ((TIMEOUT_REQUESTS++)); echo -n "T" ;;
            *) echo -n "?" ;;
        esac
    done
    
    echo ""
}

# Attack 2: Bot user agents
attack_bot_agents() {
    local count=$1
    log_info "Attack 2: Bot user-agents ($count requests)..."
    
    local agents=("curl/7.68.0" "python-requests/2.25.1" "wget/1.20" "Scrapy/2.5.0")
    
    for i in $(seq 1 $count); do
        local agent="${agents[$RANDOM % ${#agents[@]}]}"
        local status=$(curl -s --max-time 5 --socks5-hostname 127.0.0.1:$TOR_SOCKS_PORT \
            -A "$agent" \
            -o /dev/null -w "%{http_code}" \
            "http://$MIRROR/" 2>/dev/null || echo "000")
        
        ((TOTAL_REQUESTS++))
        
        case "$status" in
            200) ((SUCCESS_REQUESTS++)); echo -n "." ;;
            429) ((BLOCKED_REQUESTS++)); echo -n "R" ;;
            403) ((BLOCKED_REQUESTS++)); echo -n "B" ;;
            000) ((TIMEOUT_REQUESTS++)); echo -n "T" ;;
            *) echo -n "?" ;;
        esac
    done
    
    echo ""
}

# Attack 3: Path enumeration
attack_paths() {
    local count=$1
    log_info "Attack 3: Path enumeration ($count paths)..."
    
    local paths=(
        "/admin" "/config" "/api" "/user" "/login" 
        "/.env" "/wp-admin" "/phpmyadmin" 
        "/../../../etc/passwd" "/backup"
    )
    
    for i in $(seq 1 $count); do
        local path="${paths[$RANDOM % ${#paths[@]}]}"
        local status=$(curl -s --max-time 5 --socks5-hostname 127.0.0.1:$TOR_SOCKS_PORT \
            -o /dev/null -w "%{http_code}" \
            "http://$MIRROR$path" 2>/dev/null || echo "000")
        
        ((TOTAL_REQUESTS++))
        
        case "$status" in
            200) ((SUCCESS_REQUESTS++)); echo -n "." ;;
            404) echo -n "N" ;;
            429) ((BLOCKED_REQUESTS++)); echo -n "R" ;;
            403) ((BLOCKED_REQUESTS++)); echo -n "B" ;;
            000) ((TIMEOUT_REQUESTS++)); echo -n "T" ;;
            *) echo -n "?" ;;
        esac
    done
    
    echo ""
}

# Attack 4: Parallel circuits (simulates distributed attack)
attack_parallel() {
    local count=$1
    log_info "Attack 4: Parallel attack simulation ($count concurrent)..."
    
    # Launch background requests
    for i in $(seq 1 $count); do
        (
            local status=$(curl -s --max-time 10 --socks5-hostname 127.0.0.1:$TOR_SOCKS_PORT \
                -o /dev/null -w "%{http_code}" \
                "http://$MIRROR/" 2>/dev/null || echo "000")
            
            echo "$status" >> /tmp/fortify_stress_results.txt
        ) &
    done
    
    # Wait for all to complete
    wait
    
    # Count results
    if [ -f /tmp/fortify_stress_results.txt ]; then
        while read status; do
            ((TOTAL_REQUESTS++))
            case "$status" in
                200) ((SUCCESS_REQUESTS++)); echo -n "." ;;
                429) ((BLOCKED_REQUESTS++)); echo -n "R" ;;
                403) ((BLOCKED_REQUESTS++)); echo -n "B" ;;
                000) ((TIMEOUT_REQUESTS++)); echo -n "T" ;;
                *) echo -n "?" ;;
            esac
        done < /tmp/fortify_stress_results.txt
        
        rm /tmp/fortify_stress_results.txt
    fi
    
    echo ""
}

# Legitimate user test
test_legitimate_user() {
    log_info "Testing legitimate user access during attacks..."
    
    # Use normal user-agent and reasonable timing
    local status=$(curl -s --max-time 10 --socks5-hostname 127.0.0.1:$TOR_SOCKS_PORT \
        -A "Mozilla/5.0 (Windows NT 10.0; rv:109.0) Gecko/20100101 Firefox/115.0" \
        -o /dev/null -w "%{http_code}" \
        "http://$MIRROR/" 2>/dev/null || echo "000")
    
    if [ "$status" = "200" ] || [ "$status" = "302" ]; then
        log_success "Legitimate user can access (HTTP $status)"
        return 0
    else
        log_error "Legitimate user blocked! (HTTP $status)"
        return 1
    fi
}

# Monitor system metrics
check_metrics() {
    echo ""
    log_info "System Metrics:"
    
    # Check node metrics
    local metrics=$(curl -s http://127.0.0.1:9100/metrics 2>/dev/null)
    if [ -n "$metrics" ]; then
        echo "$metrics" | grep -E "rate_limit|violation|session" | head -10
    else
        log_error "Cannot fetch metrics (node may be down)"
    fi
}

# Main test
main() {
    clear
    echo "╔════════════════════════════════════════════════════════════════╗"
    echo "║         Fortify Automated Stress Test                         ║"
    echo "╚════════════════════════════════════════════════════════════════╝"
    echo ""
    echo "Legend: . = Success | R = Rate limited | B = Blocked"
    echo "        T = Timeout | N = Not found   | ? = Unknown"
    echo ""
    
    START_TIME=$(date +%s)
    
    # Run attacks in sequence
    attack_single_circuit 30
    sleep 1
    
    attack_bot_agents 20
    sleep 1
    
    attack_paths 15
    sleep 1
    
    attack_parallel 10
    sleep 2
    
    # Test legitimate access
    test_legitimate_user
    
    # Check system state
    check_metrics
    
    END_TIME=$(date +%s)
    ELAPSED=$((END_TIME - START_TIME))
    
    # Results
    echo ""
    echo "╔════════════════════════════════════════════════════════════════╗"
    echo "║                      Test Results                              ║"
    echo "╚════════════════════════════════════════════════════════════════╝"
    echo ""
    echo "Duration: ${ELAPSED}s"
    echo "Total Requests: $TOTAL_REQUESTS"
    echo "Success (200): $SUCCESS_REQUESTS"
    echo "Blocked (403/429): $BLOCKED_REQUESTS"
    echo "Timeouts: $TIMEOUT_REQUESTS"
    echo ""
    
    local block_rate=$((BLOCKED_REQUESTS * 100 / TOTAL_REQUESTS))
    echo "Block Rate: ${block_rate}%"
    
    if [ $block_rate -gt 20 ]; then
        log_success "Fortify is actively blocking malicious traffic"
    else
        log_error "Block rate seems low - check configuration"
    fi
    
    echo ""
    log_info "View logs: tail -f /var/log/fortify/fortify.log"
    log_info "Check sessions: curl http://127.0.0.1:8080/sessions"
}

main "$@"
