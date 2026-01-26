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

## Step 1: Install and Build (5 minutes)

```bash
# 1. Update system and install build dependencies
sudo apt update && sudo apt upgrade -y
sudo apt install -y git build-essential pkg-config libssl-dev

# 2. Clone the repository
git clone https://github.com/Nespartious/Fortify.git
cd Fortify/fortify

# 3. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# 4. Build the project (takes ~5-10 minutes)
cargo build --release

# 5. Run the TUI
./target/release/fortify
```

**☕ While it builds:** Read the [ELI5 Guide](../09-ELI5/explain-like-im-5.md) to understand how Fortify works.

**That's it!** The TUI wizard will detect and install Tor, Python, and vanguards automatically.

---

## Step 2: Configure Through TUI (3 minutes)

The TUI wizard will guide you through:
1. Backend configuration (your real .onion address)
2. Signing key generation
3. Tor setup and configuration
4. Component deployment
5. Mirror creation

Just follow the prompts - it's interactive and intuitive!

---

## Step 3: Get Your Mirror Addresses (1 minute)

**From TUI:**
- Navigate to "Mirrors" tab
- Press 'E' to export mirror addresses
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

## Step 4: Test It! (2 minutes)

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
