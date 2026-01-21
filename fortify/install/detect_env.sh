#!/bin/bash
# Environment Detection Script

# Minimum requirements
MIN_MEMORY_GB=1
MIN_DISK_GB=5
MIN_CORES=1

detect_environment() {
    echo "=== System Environment Detection ==="
    echo ""
    
    local warnings=0
    local errors=0
    
    # OS and Distribution
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS_NAME=$NAME
        OS_VERSION=$VERSION_ID
        echo "✓ OS: $OS_NAME $OS_VERSION"
        
        # Check if supported distribution
        if [[ "$ID" != "ubuntu" ]] && [[ "$ID" != "debian" ]]; then
            echo "  WARNING: Untested distribution. Ubuntu/Debian recommended."
            ((warnings++))
        fi
    else
        echo "✗ ERROR: Cannot detect OS distribution"
        ((errors++))
    fi
    
    # Kernel version
    KERNEL_VERSION=$(uname -r)
    echo "✓ Kernel: $KERNEL_VERSION"
    
    # CPU info
    CPU_CORES=$(nproc)
    CPU_MODEL=$(lscpu | grep "Model name" | cut -d: -f2 | xargs)
    echo "✓ CPU: $CPU_MODEL ($CPU_CORES cores)"
    
    if [ "$CPU_CORES" -lt "$MIN_CORES" ]; then
        echo "  WARNING: Less than $MIN_CORES CPU cores"
        ((warnings++))
    fi
    
    # Memory
    TOTAL_MEM_KB=$(grep MemTotal /proc/meminfo | awk '{print $2}')
    TOTAL_MEM_GB=$((TOTAL_MEM_KB / 1024 / 1024))
    TOTAL_MEM_HUMAN=$(free -h | awk '/^Mem:/{print $2}')
    echo "✓ Memory: $TOTAL_MEM_HUMAN (${TOTAL_MEM_GB}GB)"
    
    if [ "$TOTAL_MEM_GB" -lt "$MIN_MEMORY_GB" ]; then
        echo "  ERROR: Insufficient memory. Minimum ${MIN_MEMORY_GB}GB required."
        ((errors++))
    fi
    
    # Disk space
    DISK_AVAILABLE_KB=$(df / | awk 'NR==2{print $4}')
    DISK_AVAILABLE_GB=$((DISK_AVAILABLE_KB / 1024 / 1024))
    DISK_AVAILABLE_HUMAN=$(df -h / | awk 'NR==2{print $4}')
    echo "✓ Disk available: $DISK_AVAILABLE_HUMAN (${DISK_AVAILABLE_GB}GB)"
    
    if [ "$DISK_AVAILABLE_GB" -lt "$MIN_DISK_GB" ]; then
        echo "  ERROR: Insufficient disk space. Minimum ${MIN_DISK_GB}GB required."
        ((errors++))
    fi
    
    # Check for virtualization
    if command -v systemd-detect-virt &> /dev/null; then
        VIRT=$(systemd-detect-virt)
        if [ "$VIRT" != "none" ]; then
            echo "✓ Virtualization: $VIRT"
        else
            echo "✓ Virtualization: Bare metal"
        fi
    fi
    
    # Check for systemd
    if command -v systemctl &> /dev/null; then
        echo "✓ Init system: systemd"
    else
        echo "✗ ERROR: systemd not detected (required)"
        ((errors++))
    fi
    
    # Check if Tor is installed
    if command -v tor &> /dev/null; then
        TOR_VERSION=$(tor --version | head -n1)
        echo "✓ Tor: $TOR_VERSION"
    else
        echo "○ Tor: Not installed (will be installed)"
    fi
    
    # Check if Rust is installed
    if command -v cargo &> /dev/null; then
        RUST_VERSION=$(rustc --version)
        echo "✓ Rust: $RUST_VERSION"
    else
        echo "○ Rust: Not installed (will be installed)"
    fi
    
    # Check for iptables
    if command -v iptables &> /dev/null; then
        echo "✓ iptables: Available"
    else
        echo "  WARNING: iptables not found"
        ((warnings++))
    fi
    
    # Check internet connectivity
    if ping -c 1 -W 2 1.1.1.1 &> /dev/null; then
        echo "✓ Internet: Connected"
    else
        echo "  WARNING: No internet connectivity detected"
        ((warnings++))
    fi
    
    # Network interfaces
    echo "✓ Network interfaces:"
    ip -br addr | awk '{print "    " $1 ": " $3}'
    
    # System load
    LOAD_AVG=$(uptime | awk -F'load average:' '{print $2}')
    echo "✓ Load average:$LOAD_AVG"
    
    echo ""
    echo "=== Detection Summary ==="
    echo "Errors: $errors"
    echo "Warnings: $warnings"
    echo ""
    
    if [ "$errors" -gt 0 ]; then
        echo "✗ System does not meet minimum requirements"
        return 1
    elif [ "$warnings" -gt 0 ]; then
        echo "⚠ System meets requirements but has warnings"
        return 0
    else
        echo "✓ System meets all requirements"
        return 0
    fi
}
