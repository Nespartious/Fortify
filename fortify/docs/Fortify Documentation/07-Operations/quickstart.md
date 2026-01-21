# 🚀 Quick Start Guide

> **Get Fortify running in 10 minutes**

---

## Prerequisites

| Requirement | Version | Purpose |
|-------------|---------|---------|
| **Rust** | 1.70+ | Compilation |
| **Tor** | 0.4.7+ | Onion services |
| **Linux** | Any | Recommended OS |

---

## Step 1: Clone & Build

```bash
# Clone repository
git clone https://your-repo/fortify.git
cd fortify

# Build all binaries (release mode)
cargo build --release

# Binaries are in target/release/
ls target/release/fortify-*
```

**Expected output:**
```
fortify-controller
fortify-gate
fortify-http
fortify-node
fortify-orchestrator
```

---

## Step 2: Tor Setup

```bash
# Install Tor
sudo apt install tor

# Configure Tor control port
sudo nano /etc/tor/torrc
```

Add these lines to `/etc/tor/torrc`:

```
ControlPort 9051
CookieAuthentication 1
CookieAuthFileGroupReadable 1
```

Restart Tor:

```bash
sudo systemctl restart tor
```

---

## Step 3: Create Configuration

```bash
# Copy example config
cp config/fortify.example.toml config/fortify.toml

# Edit configuration
nano config/fortify.toml
```

**Minimal config:**

```toml
[global]
environment = "development"
log_level = "info"

[backend]
address = "127.0.0.1"
port = 8080

[proxy]
bind_address = "127.0.0.1"
bind_port = 8082

[gate]
bind_address = "127.0.0.1"
bind_port = 8081
enable_pow = false

[tor]
control_address = "127.0.0.1"
control_port = 9051
socks_port = 9050
```

---

## Step 4: Start Backend Service

Start whatever backend you're protecting:

```bash
# Example: Simple Python server
python3 -m http.server 8080

# Example: Your application
./your-backend --port 8080
```

---

## Step 5: Start Fortify

### Option A: Development Mode (Quick)

```bash
# Run the dev script
./scripts/dev-run.sh
```

### Option B: Manual Start (Recommended)

Open 4 terminals:

**Terminal 1: Controller**
```bash
./target/release/fortify-controller --config config/fortify.toml
```

**Terminal 2: Gate**
```bash
./target/release/fortify-gate --config config/fortify.toml
```

**Terminal 3: Node (Healthy)**
```bash
./target/release/fortify-node --config config/node-healthy.toml
```

**Terminal 4: HTTP Proxy**
```bash
./target/release/fortify-http --config config/fortify.toml
```

---

## Step 6: Verify Installation

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     VERIFICATION CHECKLIST                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  1. Gate Health Check                                                   │
│     ──────────────────────────────────────────────────────────────      │
│     curl http://127.0.0.1:8081/health                                   │
│     Expected: {"status":"ok"}                                           │
│                                                                         │
│  2. Proxy Health Check                                                  │
│     ──────────────────────────────────────────────────────────────      │
│     curl http://127.0.0.1:8082/health                                   │
│     Expected: {"status":"ok"}                                           │
│                                                                         │
│  3. Visit Gate Page                                                     │
│     ──────────────────────────────────────────────────────────────      │
│     Browser: http://127.0.0.1:8081/                                     │
│     Expected: Captcha verification page                                 │
│                                                                         │
│  4. Complete Captcha                                                    │
│     ──────────────────────────────────────────────────────────────      │
│     Solve captcha, get cookie                                           │
│     Check cookie: fortify_session=...                                   │
│                                                                         │
│  5. Access Backend via Proxy                                            │
│     ──────────────────────────────────────────────────────────────      │
│     Browser: http://127.0.0.1:8082/                                     │
│     Expected: Your backend content                                      │
│                                                                         │
│  6. Admin Panel                                                         │
│     ──────────────────────────────────────────────────────────────      │
│     Browser: http://127.0.0.1:8082/ctrl_8f7k3m9x2n4p1q6w5v0b8c/        │
│     Expected: Admin dashboard                                           │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Architecture Diagram

```
                    DEVELOPMENT SETUP
════════════════════════════════════════════════════════════════

    User Browser
         │
         │  HTTP Request
         ▼
  ┌─────────────────┐
  │  Gate (8081)    │  ◄── First-time visitors
  │  Captcha/PoW    │      get verified here
  └────────┬────────┘
           │
           │  Session Cookie
           ▼
  ┌─────────────────┐
  │  Proxy (8082)   │  ◄── Verified users
  │  HTTP Proxy     │      access backend here
  │  + Admin Panel  │
  └────────┬────────┘
           │
           │  Proxied Request
           ▼
  ┌─────────────────┐
  │  Node (9100)    │  ◄── Session tracking
  │  Behavioral     │      and analysis
  └────────┬────────┘
           │
           │  Filtered Request
           ▼
  ┌─────────────────┐
  │  Backend (8080) │  ◄── Your actual
  │  Your Service   │      application
  └─────────────────┘
```

---

## Common Issues

### Issue: "Connection refused to Tor"

**Solution:**
```bash
# Check Tor is running
sudo systemctl status tor

# Check control port
netstat -tlnp | grep 9051

# Fix permissions
sudo usermod -a -G debian-tor $USER
```

### Issue: "Cookie authentication failed"

**Solution:**
```bash
# Check cookie file exists
ls -la /run/tor/control.authcookie

# Set permissions
sudo chmod 640 /run/tor/control.authcookie
sudo chgrp $USER /run/tor/control.authcookie
```

### Issue: "Address already in use"

**Solution:**
```bash
# Find process using port
lsof -i :8081

# Kill it or change port
kill -9 <PID>
# OR
# Edit config to use different port
```

### Issue: "Session cookie not set"

**Solution:**
1. Visit gate directly: `http://127.0.0.1:8081/`
2. Complete captcha
3. Check browser cookies for `fortify_session`
4. Then visit proxy: `http://127.0.0.1:8082/`

---

## Next Steps

| Task | Command/Action |
|------|----------------|
| Add more nodes | Copy `node-healthy.toml`, change port |
| Enable PoW | Set `enable_pow = true` in config |
| Production mode | See [Hardening Guide](../05-Security/hardening.md) |
| Add Tor hidden service | See [Tor Integration](../03-TOR-Integration/onion-services.md) |
| Setup systemd | Copy files from `install/systemd/` |

---

## Quick Reference

| Component | Default Port | Purpose |
|-----------|--------------|---------|
| Gate | 8081 | Captcha/PoW verification |
| Proxy | 8082 | HTTP proxy to backend |
| Node (healthy) | 9100+ | Healthy user pool |
| Node (threat) | 9110+ | Suspicious user isolation |
| Tor Control | 9051 | Onion service management |
| Tor SOCKS | 9050 | Onion routing |
| Backend | 8080 | Your actual service |

---

*For production deployments, see the [Hardening Guide](../05-Security/hardening.md)*
