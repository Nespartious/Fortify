#!/bin/bash
# Fortify Installation Script
# Requires: Ubuntu/Debian, root privileges

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "======================================"
echo "    Fortify Installation System"
echo "======================================"
echo ""

# Source helper scripts
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/detect_env.sh"
source "${SCRIPT_DIR}/harden_os.sh"
source "${SCRIPT_DIR}/tor_setup.sh"

# Check prerequisites
if [[ $EUID -ne 0 ]]; then
   echo -e "${RED}ERROR: This script must be run as root${NC}"
   exit 1
fi

# Detect distribution
if [ -f /etc/debian_version ]; then
    DISTRO="debian"
elif [ -f /etc/redhat-release ]; then
    echo -e "${RED}ERROR: RedHat-based distributions not yet supported${NC}"
    exit 1
else
    echo -e "${RED}ERROR: Unsupported distribution${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Running as root on Debian-based system${NC}"
echo ""

# Phase 0: Environment Detection
echo "======================================"
echo "  Phase 0: Environment Detection"
echo "======================================"
detect_environment

if [ $? -ne 0 ]; then
    echo ""
    echo -e "${RED}System does not meet minimum requirements. Aborting.${NC}"
    exit 1
fi

echo ""
read -p "Continue with installation? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Installation cancelled."
    exit 0
fi
echo ""

# Phase 1: OS Hardening
echo "======================================"
echo "  Phase 1: OS Hardening"
echo "======================================"
harden_os

if [ $? -ne 0 ]; then
    echo ""
    echo -e "${YELLOW}WARNING: OS hardening had errors, but continuing...${NC}"
fi
echo ""

# Phase 2: Installing Dependencies
echo "======================================"
echo "  Phase 2: Installing Dependencies"
echo "======================================"
echo "Updating package lists..."
apt-get update -qq

echo "Installing required packages..."
PACKAGES="tor build-essential pkg-config libssl-dev curl"
apt-get install -y $PACKAGES

echo -e "${GREEN}✓ Dependencies installed${NC}"
echo ""

# Install Python3 and pip for vanguards
echo "Installing Python3 and pip for vanguards addon..."
apt-get install -y python3 python3-pip python3-venv
echo -e "${GREEN}✓ Python3/pip installed${NC}"
echo ""

# Install Rust if not present
if ! command -v cargo &> /dev/null; then
    echo "Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --quiet
    source "$HOME/.cargo/env"
    echo -e "${GREEN}✓ Rust toolchain installed${NC}"
else
    echo -e "${GREEN}✓ Rust toolchain already installed${NC}"
fi
echo ""

# Phase 3: Tor Configuration
echo "======================================"
echo "  Phase 3: Tor Configuration"
echo "======================================"
setup_tor

if [ $? -ne 0 ]; then
    echo ""
    echo -e "${RED}ERROR: Tor setup failed. Aborting.${NC}"
    exit 1
fi
echo ""

# Phase 3.5: Vanguards Installation
echo "======================================"
echo "  Phase 3.5: Vanguards Addon"
echo "======================================"
source "${SCRIPT_DIR}/vanguards_setup.sh"
setup_vanguards

if [ $? -ne 0 ]; then
    echo ""
    echo -e "${YELLOW}WARNING: Vanguards setup failed, but continuing...${NC}"
    echo -e "${YELLOW}Fortify will work without vanguards, but guard protection will be reduced.${NC}"
fi
echo ""

# Phase 4: Creating Fortify User
echo "======================================"
echo "  Phase 4: Creating Fortify User"
echo "======================================"
if ! id -u fortify &> /dev/null; then
    useradd -r -s /usr/sbin/nologin -d /nonexistent -c "Fortify System User" fortify
    echo -e "${GREEN}✓ Created fortify user${NC}"
else
    echo -e "${GREEN}✓ Fortify user already exists${NC}"
fi
echo ""

# Phase 5: Installing Fortify Binaries
echo "======================================"
echo "  Phase 5: Building & Installing"
echo "======================================"
cd "${SCRIPT_DIR}/.."

echo "Building Fortify workspace (this may take a few minutes)..."
if cargo build --workspace --release --quiet; then
    echo -e "${GREEN}✓ Build successful${NC}"
else
    echo -e "${RED}ERROR: Build failed${NC}"
    exit 1
fi

echo "Creating installation directories..."
mkdir -p /opt/fortify/{bin,config}
mkdir -p /etc/fortify
mkdir -p /var/log/fortify
mkdir -p /var/run/fortify

echo "Installing binaries..."
cp target/release/fortify-controller /opt/fortify/bin/ 2>/dev/null || echo "  Note: fortify-controller binary not found"
cp target/release/fortify-orchestrator /opt/fortify/bin/ 2>/dev/null || echo "  Note: fortify-orchestrator binary not found"
cp target/release/fortify-node /opt/fortify/bin/ 2>/dev/null || echo "  Note: fortify-node binary not found"

echo "Installing configuration templates..."
cp -r config/*.toml /etc/fortify/

echo "Setting permissions..."
chown -R root:fortify /opt/fortify
chmod -R 750 /opt/fortify
chown -R fortify:fortify /var/log/fortify
chmod 750 /var/log/fortify
chown -R fortify:fortify /var/run/fortify
chmod 750 /var/run/fortify
chown -R root:fortify /etc/fortify
chmod 750 /etc/fortify
chmod 640 /etc/fortify/*.toml

echo -e "${GREEN}✓ Fortify installed${NC}"
echo ""

# Phase 6: Installing Systemd Services
echo "======================================"
echo "  Phase 6: Systemd Services"
echo "======================================"
echo "Installing service units..."
cp "${SCRIPT_DIR}/systemd/"*.service /etc/systemd/system/
systemctl daemon-reload

echo "Enabling services..."
systemctl enable fortify-controller.service
systemctl enable fortify-orchestrator.service
systemctl enable fortify-node-healthy.service
systemctl enable fortify-node-threat.service

echo -e "${GREEN}✓ Services installed and enabled${NC}"
echo ""

# Installation Complete
echo "======================================"
echo "  Installation Complete!"
echo "======================================"
echo ""
echo -e "${GREEN}Fortify has been successfully installed.${NC}"
echo ""
echo "Next steps:"
echo ""
echo "1. Configure your deployment:"
echo "   cp /etc/fortify/fortify.example.toml /etc/fortify/fortify.toml"
echo "   nano /etc/fortify/fortify.toml"
echo ""
echo "2. Set your real service onion address in the config"
echo ""
echo "3. Start Fortify:"
echo "   systemctl start fortify-controller"
echo ""
echo "4. Check status:"
echo "   systemctl status fortify-controller"
echo "   journalctl -u fortify-controller -f"
echo ""
echo -e "${YELLOW}IMPORTANT SECURITY NOTES:${NC}"
echo "- Review ${SCRIPT_DIR}/../docs/hardening.md for additional hardening"
echo "- Configure full iptables firewall rules"
echo "- Consider enabling SELinux or AppArmor"
echo "- Set up monitoring and alerting"
echo "- Never run other services on this host"
echo ""
echo "Documentation: ${SCRIPT_DIR}/../docs/"
echo ""
