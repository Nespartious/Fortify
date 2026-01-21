#!/bin/bash
# Tor Setup Script

setup_tor() {
    echo "=== Configuring Tor Daemon ==="
    echo ""
    
    # Check if Tor is installed
    if ! command -v tor &> /dev/null; then
        echo "✗ ERROR: Tor is not installed"
        return 1
    fi
    
    # Check Tor version
    TOR_VERSION=$(tor --version | head -n1 | awk '{print $3}')
    echo "✓ Found Tor version: $TOR_VERSION"
    
    # Detect Tor user (varies by distribution)
    if id debian-tor &> /dev/null 2>&1; then
        TOR_USER="debian-tor"
    elif id _tor &> /dev/null 2>&1; then
        TOR_USER="_tor"
    elif id tor &> /dev/null 2>&1; then
        TOR_USER="tor"
    else
        echo "⚠ WARNING: Cannot detect Tor user, assuming debian-tor"
        TOR_USER="debian-tor"
    fi
    echo "✓ Tor user: $TOR_USER"
    
    # Backup existing torrc
    if [ -f /etc/tor/torrc ]; then
        BACKUP_FILE="/etc/tor/torrc.backup-$(date +%Y%m%d-%H%M%S)"
        cp /etc/tor/torrc "$BACKUP_FILE"
        echo "✓ Backed up existing torrc to $BACKUP_FILE"
    fi
    
    # Copy Fortify torrc template
    if cp "${SCRIPT_DIR}/templates/torrc.template" /etc/tor/torrc.fortify; then
        echo "✓ Installed Fortify Tor configuration"
    else
        echo "✗ ERROR: Failed to install Tor configuration"
        return 1
    fi
    
    # Add include directive to main torrc if not already present
    if ! grep -q "torrc.fortify" /etc/tor/torrc 2>/dev/null; then
        echo "" >> /etc/tor/torrc
        echo "# Fortify configuration" >> /etc/tor/torrc
        echo "%include /etc/tor/torrc.fortify" >> /etc/tor/torrc
        echo "✓ Added Fortify config include to torrc"
    else
        echo "○ Fortify config already included in torrc"
    fi
    
    # Ensure Tor data directory exists with correct permissions
    mkdir -p /var/lib/tor
    chown -R ${TOR_USER}:${TOR_USER} /var/lib/tor
    chmod 700 /var/lib/tor
    echo "✓ Tor data directory configured"
    
    # Create hidden service directory structure (will be populated by controller)
    mkdir -p /var/lib/tor/fortify-orchestrator
    chown -R ${TOR_USER}:${TOR_USER} /var/lib/tor/fortify-orchestrator
    chmod 700 /var/lib/tor/fortify-orchestrator
    echo "✓ Hidden service directories created"
    
    # Validate Tor configuration
    echo "Validating Tor configuration..."
    if tor --verify-config > /dev/null 2>&1; then
        echo "✓ Tor configuration valid"
    else
        echo "⚠ WARNING: Tor configuration validation failed"
        echo "  Run 'tor --verify-config' to see details"
    fi
    
    # Restart Tor to apply changes
    echo "Restarting Tor daemon..."
    if systemctl restart tor; then
        echo "✓ Tor daemon restarted"
    else
        echo "✗ ERROR: Failed to restart Tor daemon"
        return 1
    fi
    
    # Wait for Tor to start
    echo "Waiting for Tor to initialize..."
    local retries=10
    while [ $retries -gt 0 ]; do
        if systemctl is-active tor &> /dev/null; then
            echo "✓ Tor daemon is running"
            
            # Check if control port is responsive
            sleep 2
            if nc -z 127.0.0.1 9051 &> /dev/null 2>&1 || timeout 1 bash -c "echo QUIT | nc 127.0.0.1 9051" &> /dev/null; then
                echo "✓ Tor control port responding"
                return 0
            fi
        fi
        sleep 1
        ((retries--))
    done
    
    # Check status and logs if startup failed
    if ! systemctl is-active tor &> /dev/null; then
        echo "✗ ERROR: Tor daemon failed to start"
        echo ""
        echo "Last 10 lines of Tor log:"
        journalctl -u tor -n 10 --no-pager
        return 1
    fi
    
    echo "✓ Tor setup complete"
    return 0
}
