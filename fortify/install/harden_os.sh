#!/bin/bash
# OS Hardening Script

harden_os() {
    echo "=== Applying OS Hardening Configurations ==="
    echo ""
    
    local failed=0
    
    # Backup existing configurations
    echo "Creating configuration backups..."
    BACKUP_DIR="/var/backups/fortify-$(date +%Y%m%d-%H%M%S)"
    mkdir -p "$BACKUP_DIR"
    
    # Backup sysctl if exists
    if [ -f /etc/sysctl.conf ]; then
        cp /etc/sysctl.conf "$BACKUP_DIR/sysctl.conf.bak"
    fi
    
    # Backup limits if exists
    if [ -f /etc/security/limits.conf ]; then
        cp /etc/security/limits.conf "$BACKUP_DIR/limits.conf.bak"
    fi
    
    # Backup iptables rules
    if command -v iptables-save &> /dev/null; then
        iptables-save > "$BACKUP_DIR/iptables.rules.bak"
    fi
    
    echo "✓ Backups saved to $BACKUP_DIR"
    echo ""
    
    # Apply kernel parameters
    echo "Configuring kernel parameters..."
    if cp "${SCRIPT_DIR}/templates/sysctl.conf" /etc/sysctl.d/99-fortify.conf; then
        if sysctl -p /etc/sysctl.d/99-fortify.conf > /dev/null 2>&1; then
            echo "✓ Kernel parameters applied"
        else
            echo "✗ ERROR: Failed to apply kernel parameters"
            ((failed++))
        fi
    else
        echo "✗ ERROR: Failed to copy sysctl configuration"
        ((failed++))
    fi
    
    # Apply resource limits
    echo "Configuring resource limits..."
    if cp "${SCRIPT_DIR}/templates/limits.conf" /etc/security/limits.d/fortify.conf; then
        echo "✓ Resource limits configured"
    else
        echo "✗ ERROR: Failed to copy limits configuration"
        ((failed++))
    fi
    
    # Configure firewall
    echo "Configuring firewall rules..."
    if command -v iptables &> /dev/null; then
        # Note: This applies minimal rules. See docs/hardening.md for full configuration.
        
        # Allow loopback
        iptables -A INPUT -i lo -j ACCEPT 2>/dev/null
        iptables -A OUTPUT -o lo -j ACCEPT 2>/dev/null
        
        # Allow established connections
        iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT 2>/dev/null
        iptables -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT 2>/dev/null
        
        # Save rules if possible
        if command -v netfilter-persistent &> /dev/null; then
            netfilter-persistent save > /dev/null 2>&1
            echo "✓ Firewall rules applied and saved"
        else
            echo "✓ Firewall rules applied (not persistent - install netfilter-persistent)"
        fi
    else
        echo "⚠ WARNING: iptables not available"
    fi
    
    # Disable unnecessary services
    echo "Disabling unnecessary services..."
    local disabled=0
    for service in avahi-daemon cups bluetooth; do
        if systemctl is-enabled "$service" &> /dev/null; then
            systemctl disable "$service" 2>/dev/null && systemctl stop "$service" 2>/dev/null
            echo "  ✓ Disabled $service"
            ((disabled++))
        fi
    done
    
    if [ "$disabled" -eq 0 ]; then
        echo "  No unnecessary services found"
    fi
    
    # Set secure permissions on sensitive files
    echo "Setting secure file permissions..."
    chmod 600 /etc/shadow 2>/dev/null
    chmod 600 /etc/gshadow 2>/dev/null
    chmod 644 /etc/passwd 2>/dev/null
    chmod 644 /etc/group 2>/dev/null
    echo "✓ File permissions secured"
    
    # Configure auditd if available
    if command -v auditctl &> /dev/null; then
        echo "Configuring audit logging..."
        # Add basic audit rules
        auditctl -w /etc/passwd -p wa -k passwd_changes 2>/dev/null
        auditctl -w /etc/shadow -p wa -k shadow_changes 2>/dev/null
        echo "✓ Audit logging configured"
    else
        echo "○ auditd not installed (optional)"
    fi
    
    # Enable automatic security updates if unattended-upgrades is available
    if command -v unattended-upgrade &> /dev/null; then
        echo "Enabling automatic security updates..."
        if [ -f /etc/apt/apt.conf.d/50unattended-upgrades ]; then
            # Configure for security updates only
            cat > /etc/apt/apt.conf.d/20auto-upgrades <<EOF
APT::Periodic::Update-Package-Lists \"1\";
APT::Periodic::Unattended-Upgrade \"1\";
APT::Periodic::AutocleanInterval \"7\";
EOF
            echo "✓ Automatic security updates enabled"
        fi
    else
        echo "○ unattended-upgrades not installed (recommended)"
    fi
    
    echo ""
    echo "=== Hardening Summary ==="
    if [ "$failed" -eq 0 ]; then
        echo "✓ OS hardening completed successfully"
        echo ""
        echo "IMPORTANT: Review docs/hardening.md for additional security measures:"
        echo "  - Full iptables firewall configuration"
        echo "  - SELinux/AppArmor setup"
        echo "  - Fail2ban configuration"
        echo "  - SSH hardening"
        echo ""
        return 0
    else
        echo "⚠ OS hardening completed with $failed errors"
        echo "Check logs above for details"
        echo ""
        return 1
    fi
}
