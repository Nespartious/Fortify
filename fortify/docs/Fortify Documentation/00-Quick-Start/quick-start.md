# 🎓 Quick Start Guide

> **Get Fortify Running in 15 Minutes**

**For:** First-time users and evaluators  
**Time:** 15-30 minutes  
**Difficulty:** Beginner

---

## What You'll Need

- Ubuntu 20.04+ or Debian 11+ server
- Root/sudo access
- 2+ GB RAM, 2+ CPU cores
- Basic command line knowledge

---

## Step 1: Install Dependencies (5 minutes)

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install dependencies
sudo apt install -y build-essential pkg-config libssl-dev tor git

# Start Tor
sudo systemctl start tor
sudo systemctl enable tor
```

---

## Step 2: Get Fortify (2 minutes)

```bash
# Clone repository
git clone https://github.com/your-org/fortify.git
cd fortify

# Build (this takes ~5-10 minutes)
cargo build --release --workspace
```

**☕ While it builds:** Read the [ELI5 Guide](../09-ELI5/explain-like-im-5.md) to understand how Fortify works.

---

## Step 3: Quick Configuration (3 minutes)

```bash
# Create config directory
sudo mkdir -p /etc/fortify

# Copy example config
sudo cp config/fortify.example.toml /etc/fortify/fortify.toml

# Generate signing key
sudo openssl rand -hex 32 > /etc/fortify/gate-signing.key
sudo chmod 600 /etc/fortify/gate-signing.key

# Edit config
sudo nano /etc/fortify/fortify.toml
```

**Change these lines:**
```toml
[service]
# YOUR ACTUAL BACKEND .ONION ADDRESS
real_onion_address = "http://your-real-backend.onion"

[orchestrator]
# Verify this matches your Tor setup
tor_cookie_path = "/var/lib/tor/control_auth_cookie"

[gate]
# Point to the key you just created
token_signing_key = "/etc/fortify/gate-signing.key"
```

Save and exit (Ctrl+X, Y, Enter).

---

## Step 4: Launch Fortify (2 minutes)

**Option A: Using TUI (Recommended)**

```bash
./target/release/fortify
```

Navigate through the wizard, it will:
1. Validate your configuration
2. Start all components
3. Create initial mirrors
4. Show you the mirror addresses

**Option B: Manual Start (Development)**

```bash
# Simple development mode
./scripts/dev-run.sh
```

---

## Step 5: Get Your Mirror Addresses (1 minute)

**From TUI:**
- Navigate to "Mirrors" tab
- Copy the .onion addresses

**From Logs:**
```bash
sudo grep "Mirror created" /var/log/fortify/fortify.log | tail -5
```

You should see something like:
```
✅ Mirror created: abc123def456ghi789.onion - ACTIVE
✅ Mirror created: xyz789uvw456rst123.onion - ACTIVE
✅ Mirror created: qwe456rty789uio123.onion - ACTIVE
```

---

## Step 6: Test It! (2 minutes)

**Test from Tor Browser:**

1. Open Tor Browser
2. Navigate to one of your mirror addresses:
   ```
   http://abc123def456ghi789.onion/
   ```

3. You should see the CAPTCHA gate:
   - Landing page with "Initialize Handshake" button
   - CAPTCHA challenge
   - After solving: Access to your real service ✅

**Test CAPTCHA solving:**
```
1. Click "Initialize Handshake"
2. Solve the CAPTCHA
3. You should be redirected to your real service
4. Your session is now VERIFIED!
```

---

## What Just Happened?

```
┌────────────────────────────────────────────────────────────┐
│                    YOUR FORTIFY SETUP                       │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Internet → Mirror (.onion) → Fortify → Real Service      │
│                                                             │
│  Components Running:                                       │
│   ✓ Controller  - Managing resources                      │
│   ✓ Orchestrator - Managing mirrors                       │
│   ✓ HTTP Proxy   - Routing requests                        │
│   ✓ Gate         - Verifying users                         │
│   ✓ Healthy Nodes - Serving verified users                │
│   ✓ Threat Nodes - Monitoring suspicious users            │
│                                                             │
│  Mirrors Active: 3                                         │
│  Status: PROTECTING YOUR SERVICE ✅                        │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

---

## Quick Checks

**Is everything running?**
```bash
ps aux | grep fortify
# Should see: controller, orchestrator, proxy, gate, nodes
```

**Check the logs:**
```bash
sudo tail -f /var/log/fortify/fortify.log
```

**Access admin panel:**
```
http://your-mirror.onion/ctrl_8f7k3m9x2n4p1q6w5v0b8c
Password: pleaseletmein123
```

---

## Next Steps

Now that Fortify is running:

1. **Understand the system:** Read [Architecture Overview](../01-Architecture/overview.md)
2. **Learn trust levels:** Read [Trust Tiers](../02-Core-Concepts/trust-tiers.md)
3. **Tune security:** Read [Configuration Reference](../05-Configuration/configuration-reference.md)
4. **Monitor it:** Read [Operations Guide](../06-Operations/monitoring.md)
5. **Production hardening:** Read [Deployment Guide](../04-Deployment/deployment-guide.md#security-hardening)

---

## Common Quick Start Issues

### "Tor connection failed"
```bash
sudo systemctl status tor
sudo systemctl start tor
```

### "Port already in use"
```bash
# Find what's using port 8082
sudo netstat -tulpn | grep :8082
# Kill it or change Fortify's port in config
```

### "Can't find signing key"
```bash
# Generate it
sudo openssl rand -hex 32 > /etc/fortify/gate-signing.key
sudo chmod 600 /etc/fortify/gate-signing.key
```

### "Backend timeout"
```bash
# Test your backend
torify curl http://your-real-service.onion/
```

For more issues, see [Troubleshooting Guide](../07-Troubleshooting/common-issues.md).

---

## Production Deployment

**This quick start is for TESTING only.**

For production:
1. Use proper systemd services (not dev-run.sh)
2. Set up log rotation
3. Configure firewalls
4. Enable vanguards
5. Harden OS
6. Change admin password
7. Set up monitoring

See [Deployment Guide](../04-Deployment/deployment-guide.md) for full production setup.

---

## Summary

✅ **You now have:**
- Fortify running and protecting your service
- 3 public mirror .onion addresses
- CAPTCHA verification for new users
- Trust-based routing for verified users
- Behavioral analysis detecting attacks
- Admin panel for monitoring

🎉 **Congratulations!** Your Tor hidden service is now protected by Fortify.

---

*Need help? Check [Troubleshooting](../07-Troubleshooting/common-issues.md) or the [ELI5 Guide](../09-ELI5/explain-like-im-5.md)*
