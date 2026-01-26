# 🚀 Deployment Guide

> **Setting Up Fortify in Production**

**Status:** 🟡 IN PROGRESS - TUI wizard is the recommended deployment method  
**Last Updated:** January 25, 2026

---

## Overview

Fortify can be deployed using two methods:
1. **TUI Wizard** (Recommended) - Interactive deployment interface
2. **Manual Installation** - For advanced users or custom setups

---

## Prerequisites

### System Requirements

**Minimum:**
- **OS:** Ubuntu 20.04+ or Debian 11+
- **CPU:** 2 cores
- **RAM:** 2 GB
- **Disk:** 10 GB free
- **Network:** Stable internet connection

**Recommended:**
- **OS:** Ubuntu 22.04 LTS
- **CPU:** 4+ cores
- **RAM:** 4+ GB
- **Disk:** 20+ GB SSD
- **Network:** High bandwidth, low latency

### Required Software

- **Rust:** Latest stable (automatically installed)
- **Tor:** 0.4.7+ (automatically configured)
- **Vanguards:** Latest (optional but recommended)
- **Build tools:** gcc, make, pkg-config

---

## Method 1: TUI Wizard (Recommended)

> 🚧 **NOTE:** The TUI wizard is still in active development (40% complete as of Jan 2026).  
> Some features may be incomplete or require manual configuration.

### Quick Start

```bash
# 1. Clone the repository
git clone https://github.com/your-org/fortify.git
cd fortify

# 2. Build Fortify
cargo build --release --workspace

# 3. Run TUI wizard
./target/release/fortify
```

### TUI Features

The TUI wizard provides:
- ✅ **Unified deployment workflow** - Step-by-step setup
- ✅ **Configuration wizard** - Interactive config generation
- ✅ **Real-time log monitoring** - Watch system activity
- ✅ **Mirror status tracking** - Monitor active mirrors
- ✅ **One-click export** - Export mirror addresses
- 🟡 **Auto-configuration** - (In progress) Automatic optimal settings
- 🟡 **Health monitoring** - (In progress) Component health dashboard

### TUI Workflow

```
┌────────────────────────────────────────────────────────────┐
│              FORTIFY TUI DEPLOYMENT WIZARD                  │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Step 1: System Check                                      │
│    ├─ Verify OS and dependencies                           │
│    ├─ Check available resources                            │
│    └─ Detect existing Tor installation                     │
│                                                             │
│  Step 2: Configuration                                     │
│    ├─ Backend .onion address                               │
│    ├─ Resource limits (nodes, orchestrators)               │
│    ├─ Security settings (vanguards, PoW)                   │
│    └─ Behavioral analysis thresholds                       │
│                                                             │
│  Step 3: Tor Setup                                         │
│    ├─ Install/configure Tor                                │
│    ├─ Setup vanguards (optional)                           │
│    └─ Generate control authentication                      │
│                                                             │
│  Step 4: Deployment                                        │
│    ├─ Start Controller                                     │
│    ├─ Spawn initial Orchestrator                           │
│    ├─ Create mirror pool                                   │
│    └─ Launch Gate and Proxy                                │
│                                                             │
│  Step 5: Verification                                      │
│    ├─ Test mirror connectivity                             │
│    ├─ Verify CAPTCHA generation                            │
│    └─ Check backend routing                                │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

---

## Method 2: Manual Installation

For advanced users who want full control over the installation process.

### Step 1: Install Dependencies

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install build dependencies
sudo apt install -y build-essential pkg-config libssl-dev

# Install Tor
sudo apt install -y tor

# (Optional) Install vanguards
sudo pip3 install vanguards
```

### Step 2: Clone and Build Fortify

```bash
# Clone repository
git clone https://github.com/your-org/fortify.git
cd fortify

# Build all components
cargo build --release --workspace

# Verify binaries
ls -lh target/release/fortify-*
```

### Step 3: Configure Tor

Use the provided installation script:

```bash
cd install
sudo ./tor_setup.sh
```

Or manually configure `/etc/tor/torrc`:

```toml
# Fortify Tor Configuration
SocksPort 9050
ControlPort 9051
CookieAuthentication 1
CookieAuthFile /var/lib/tor/control_auth_cookie

# Enable PoW defense (optional but recommended)
HiddenServicePoWDefensesEnabled 1
HiddenServicePoWQueueRate 250
HiddenServicePoWQueueBurst 2500

# Directory for hidden services
HiddenServiceDir /var/lib/tor/fortify_mirrors/
```

Restart Tor:
```bash
sudo systemctl restart tor
sudo systemctl status tor
```

### Step 4: Configure Fortify

Copy and customize the configuration file:

```bash
# Create config directory
sudo mkdir -p /etc/fortify

# Copy example config
sudo cp config/fortify.example.toml /etc/fortify/fortify.toml

# Edit configuration
sudo nano /etc/fortify/fortify.toml
```

**Required Configuration Changes:**

```toml
[service]
# YOUR REAL BACKEND .ONION ADDRESS
real_onion_address = "http://your-real-service.onion"
real_service_port = 80

[orchestrator]
# Tor control settings (verify paths)
tor_control_port = "127.0.0.1:9051"
tor_cookie_path = "/var/lib/tor/control_auth_cookie"

[gate]
# Generate a signing key
token_signing_key = "/etc/fortify/gate-signing.key"

[logging]
# Set appropriate log level
level = "info"
log_file = "/var/log/fortify/fortify.log"
```

Generate signing key:
```bash
sudo openssl rand -hex 32 > /etc/fortify/gate-signing.key
sudo chmod 600 /etc/fortify/gate-signing.key
```

### Step 5: Setup Vanguards (Optional but Recommended)

```bash
cd install
sudo ./vanguards_setup.sh
```

Or manually:
```bash
# Install vanguards
sudo pip3 install vanguards

# Create config
sudo mkdir -p /etc/vanguards
sudo cp install/templates/vanguards.conf.template /etc/vanguards/vanguards.conf

# Configure environment
export VANGUARDS_ENABLED=true
export VANGUARDS_LAYER2_GUARDS=4
export VANGUARDS_LAYER3_GUARDS=8
```

### Step 6: Deploy Fortify

**Option A: Development Mode** (for testing)

```bash
# Simple development run (deprecated, use TUI)
./scripts/dev-run.sh --wipe
```

**Option B: Production Mode** (systemd services)

> 🚧 **TODO:** Systemd service files are located in `install/systemd/` but need documentation.  
> For now, refer to the TUI wizard or manual process below.

Create systemd service:
```bash
# Create service file
sudo nano /etc/systemd/system/fortify-controller.service
```

```ini
[Unit]
Description=Fortify Controller
After=network.target tor.service

[Service]
Type=simple
User=fortify
Group=fortify
ExecStart=/usr/local/bin/fortify-controller --config /etc/fortify/fortify.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable fortify-controller
sudo systemctl start fortify-controller
sudo systemctl status fortify-controller
```

---

## Post-Deployment

### Verify Installation

```bash
# Check Controller is running
ps aux | grep fortify-controller

# Check Tor is running
sudo systemctl status tor

# Check logs
sudo tail -f /var/log/fortify/fortify.log
```

### Get Mirror Addresses

Using TUI:
```bash
./target/release/fortify
# Navigate to "Mirrors" tab
# Click "Export Addresses"
```

Or check logs:
```bash
sudo grep "Mirror created" /var/log/fortify/fortify.log
```

### Test Connectivity

```bash
# Test mirror (replace with your actual .onion)
torify curl http://your-mirror.onion/

# Should redirect to Gate for CAPTCHA
```

---

## Deployment Scenarios

### Small Site (Low Traffic)

```toml
[controller]
max_orchestrators = 2
max_healthy_nodes = 3
max_threat_nodes = 2

[http_proxy]
max_concurrent_connections = 100

[gate]
max_concurrent_verifications = 5
```

**Resources:** ~1 GB RAM, 1-2 CPU cores

---

### Medium Site (Moderate Traffic)

```toml
[controller]
max_orchestrators = 3
max_healthy_nodes = 5
max_threat_nodes = 3

[http_proxy]
max_concurrent_connections = 500

[gate]
max_concurrent_verifications = 10
```

**Resources:** ~2-4 GB RAM, 2-4 CPU cores

---

### Large Site (High Traffic)

```toml
[controller]
max_orchestrators = 5
max_healthy_nodes = 10
max_threat_nodes = 5

[http_proxy]
max_concurrent_connections = 1000

[gate]
max_concurrent_verifications = 20
```

**Resources:** ~4-8 GB RAM, 4-8 CPU cores

---

## Security Hardening

### OS Hardening

Use the provided script:
```bash
cd install
sudo ./harden_os.sh
```

This script:
- Disables unnecessary services
- Configures firewall rules
- Enables automatic security updates
- Hardens SSH configuration
- Sets up fail2ban

### File Permissions

```bash
# Fortify directories
sudo chown -R fortify:fortify /etc/fortify
sudo chmod 700 /etc/fortify
sudo chmod 600 /etc/fortify/*.key

# Log directory
sudo mkdir -p /var/log/fortify
sudo chown fortify:fortify /var/log/fortify
sudo chmod 750 /var/log/fortify
```

### Firewall Configuration

```bash
# Allow only necessary ports
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow ssh
sudo ufw enable
```

---

## Troubleshooting Deployment

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| "Can't connect to Tor control port" | Tor not running | `sudo systemctl start tor` |
| "Permission denied: control_auth_cookie" | Wrong file permissions | `sudo chmod 640 /var/lib/tor/control_auth_cookie` |
| "Port already in use" | Another service using port | Check with `netstat -tulpn` |
| "Failed to create mirror" | Tor configuration issue | Check `/var/log/tor/log` |

### Debug Mode

Run with verbose logging:
```bash
RUST_LOG=debug ./target/release/fortify-controller
```

---

## Updates and Maintenance

### Updating Fortify

```bash
# Pull latest code
git pull origin main

# Rebuild
cargo build --release --workspace

# Restart services
sudo systemctl restart fortify-controller
```

### Backup Configuration

```bash
# Backup config and keys
sudo tar -czf fortify-backup-$(date +%Y%m%d).tar.gz \
    /etc/fortify/ \
    /var/log/fortify/
```

---

## Next Steps

After deployment:
1. Review [Operations Guide](../06-Operations/monitoring.md) for monitoring setup
2. Configure [Behavioral Analysis](../02-Core-Concepts/behavioral-analysis.md) thresholds
3. Set up [Admin Panel](../08-API-Reference/api-reference.md#admin-api) access
4. Test with [Troubleshooting Guide](../07-Troubleshooting/common-issues.md)

---

> 🚧 **TODO:** This guide will be updated as the TUI wizard reaches completion.  
> Current TUI status: 40% complete (as of January 2026)

*For additional help, see [Troubleshooting Guide](../07-Troubleshooting/common-issues.md)*
