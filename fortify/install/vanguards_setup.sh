#!/bin/bash
# Vanguards Addon Setup Script
# Installs and configures the Tor vanguards addon for Fortify
#
# Vanguards provides additional guard layers to protect against
# guard discovery and deanonymization attacks - a critical defense
# for Tor hidden services.

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

setup_vanguards() {
    echo "======================================"
    echo "  Setting Up Vanguards Addon"
    echo "======================================"
    echo ""
    echo "Vanguards provides Layer 2/3 guard protection against:"
    echo "  • Guard discovery attacks"
    echo "  • Deanonymization via Sybil attacks"
    echo "  • Circuit manipulation attacks"
    echo ""
    
    # Check if Python3 is available
    if ! command -v python3 &> /dev/null; then
        echo -e "${RED}✗ ERROR: Python3 is required for vanguards${NC}"
        echo "  Install with: sudo apt install python3 python3-pip"
        return 1
    fi
    
    PYTHON_VERSION=$(python3 --version 2>&1 | awk '{print $2}')
    echo -e "${GREEN}✓ Found Python: $PYTHON_VERSION${NC}"
    
    # Check if pip is available
    if ! command -v pip3 &> /dev/null && ! python3 -m pip --version &> /dev/null 2>&1; then
        echo -e "${RED}✗ ERROR: pip3 is required${NC}"
        echo "  Install with: sudo apt install python3-pip"
        return 1
    fi
    
    # Check if vanguards is already installed
    if command -v vanguards &> /dev/null; then
        echo -e "${GREEN}✓ Vanguards already installed: $(which vanguards)${NC}"
        return 0
    fi
    
    if python3 -c "import vanguards" 2>/dev/null; then
        echo -e "${GREEN}✓ Vanguards already available as Python module${NC}"
        return 0
    fi
    
    # Install vanguards via pip
    echo "Installing vanguards addon..."
    
    # Try system-wide install first (for production)
    if [ "$EUID" -eq 0 ]; then
        # For root, try system pip first
        if pip3 install vanguards 2>/dev/null; then
            echo -e "${GREEN}✓ Vanguards installed system-wide via pip${NC}"
        else
            # Try with --break-system-packages for modern Python
            if pip3 install --break-system-packages vanguards 2>/dev/null; then
                echo -e "${GREEN}✓ Vanguards installed system-wide${NC}"
            else
                # Use venv as fallback
                install_via_venv "/opt/fortify/venv"
            fi
        fi
    else
        # User install for non-root
        if pip3 install --user vanguards 2>/dev/null; then
            echo -e "${GREEN}✓ Vanguards installed via pip (user mode)${NC}"
            
            # Add ~/.local/bin to PATH hint
            if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
                echo -e "${YELLOW}Note: Add ~/.local/bin to your PATH:${NC}"
                echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
            fi
        else
            # Use venv as fallback
            install_via_venv "/tmp/fortify/venv"
        fi
    fi
    
    # Verify vanguards is available
    verify_vanguards_install
    
    # Create vanguards directories
    mkdir -p /tmp/fortify/vanguards
    mkdir -p /tmp/fortify/config
    
    # Copy configuration template if exists
    if [ -f "${SCRIPT_DIR}/templates/vanguards.conf.template" ]; then
        cp "${SCRIPT_DIR}/templates/vanguards.conf.template" /tmp/fortify/config/vanguards.conf.template
        echo -e "${GREEN}✓ Vanguards configuration template copied${NC}"
    fi
    
    echo ""
    echo "======================================"
    echo "  Vanguards Setup Complete"
    echo "======================================"
    echo ""
    echo "Vanguards will be started automatically by the Fortify controller."
    echo ""
    echo "Configuration via environment variables:"
    echo "  VANGUARDS_ENABLED=true          # Enable/disable vanguards"
    echo "  VANGUARDS_LAYER2_GUARDS=4       # Number of layer2 guards"
    echo "  VANGUARDS_LAYER3_GUARDS=8       # Number of layer3 guards"
    echo "  VANGUARDS_CIRC_MAX_AGE_HOURS=24 # Max circuit age"
    echo ""
    
    return 0
}

install_from_git() {
    echo -e "${YELLOW}⚠ Pip install failed, trying from git...${NC}"
    VANGUARDS_DIR="/opt/vanguards"
    
    if [ -d "$VANGUARDS_DIR" ]; then
        echo "  Updating existing installation..."
        cd "$VANGUARDS_DIR"
        git pull origin main 2>/dev/null || git pull origin master 2>/dev/null || true
    else
        echo "  Cloning vanguards repository..."
        if [ "$EUID" -eq 0 ]; then
            git clone https://github.com/mikeperry-tor/vanguards.git "$VANGUARDS_DIR" 2>/dev/null
        else
            sudo git clone https://github.com/mikeperry-tor/vanguards.git "$VANGUARDS_DIR" 2>/dev/null
        fi
    fi
    
    if [ -d "$VANGUARDS_DIR" ]; then
        cd "$VANGUARDS_DIR"
        if [ "$EUID" -eq 0 ]; then
            python3 setup.py install 2>/dev/null || pip3 install . 2>/dev/null || true
        else
            sudo python3 setup.py install 2>/dev/null || pip3 install --user . 2>/dev/null || true
        fi
        echo -e "${GREEN}✓ Vanguards installed from git${NC}"
    else
        echo -e "${RED}✗ ERROR: Failed to clone vanguards repository${NC}"
        return 1
    fi
}

install_via_venv() {
    local venv_dir="$1"
    echo "Creating virtual environment for vanguards at $venv_dir..."
    
    # Ensure python3-venv is available
    if ! python3 -m venv --help &>/dev/null; then
        echo -e "${YELLOW}Installing python3-venv...${NC}"
        if [ "$EUID" -eq 0 ]; then
            apt-get install -y python3-venv 2>/dev/null || true
        else
            sudo apt-get install -y python3-venv 2>/dev/null || true
        fi
    fi
    
    # Create venv directory
    mkdir -p "$(dirname "$venv_dir")"
    
    if python3 -m venv "$venv_dir" 2>/dev/null; then
        if "$venv_dir/bin/pip" install vanguards 2>/dev/null; then
            echo -e "${GREEN}✓ Vanguards installed in virtual environment${NC}"
            echo "  Vanguards binary: $venv_dir/bin/vanguards"
            return 0
        fi
    fi
    
    echo -e "${YELLOW}⚠ Virtual environment install failed, trying git...${NC}"
    install_from_git
}

verify_vanguards_install() {
    if command -v vanguards &> /dev/null; then
        VANGUARDS_PATH=$(which vanguards)
        echo -e "${GREEN}✓ Vanguards binary found: $VANGUARDS_PATH${NC}"
        return 0
    fi
    
    if python3 -c "import vanguards" 2>/dev/null; then
        echo -e "${GREEN}✓ Vanguards available as Python module${NC}"
        return 0
    fi
    
    # Check ~/.local/bin
    if [ -f "$HOME/.local/bin/vanguards" ]; then
        echo -e "${GREEN}✓ Vanguards found at: $HOME/.local/bin/vanguards${NC}"
        return 0
    fi
    
    # Check /opt/fortify/venv (production)
    if [ -f "/opt/fortify/venv/bin/vanguards" ]; then
        echo -e "${GREEN}✓ Vanguards found at: /opt/fortify/venv/bin/vanguards${NC}"
        return 0
    fi
    
    # Check /tmp/fortify/venv (dev)
    if [ -f "/tmp/fortify/venv/bin/vanguards" ]; then
        echo -e "${GREEN}✓ Vanguards found at: /tmp/fortify/venv/bin/vanguards${NC}"
        return 0
    fi
    
    # Check /opt/vanguards
    if [ -f "/opt/vanguards/src/vanguards.py" ]; then
        echo -e "${GREEN}✓ Vanguards found at: /opt/vanguards${NC}"
        return 0
    fi
    
    echo -e "${YELLOW}⚠ WARNING: Vanguards installed but location unclear${NC}"
    echo "  The Fortify controller will attempt to locate it at runtime."
    return 0
}

# Check vanguards status
check_vanguards() {
    if pgrep -f "vanguards" > /dev/null 2>&1; then
        PID=$(pgrep -f "vanguards" | head -1)
        echo "✓ Vanguards is running (PID: $PID)"
        return 0
    else
        echo "○ Vanguards is not running"
        return 1
    fi
}

# Stop vanguards
stop_vanguards() {
    if pgrep -f "vanguards" > /dev/null 2>&1; then
        echo "Stopping vanguards..."
        pkill -f "vanguards" || true
        sleep 1
        echo "✓ Vanguards stopped"
    fi
}

# Run setup if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    case "${1:-setup}" in
        setup)
            setup_vanguards
            ;;
        check)
            check_vanguards
            ;;
        stop)
            stop_vanguards
            ;;
        *)
            echo "Usage: $0 {setup|check|stop}"
            exit 1
            ;;
    esac
fi
