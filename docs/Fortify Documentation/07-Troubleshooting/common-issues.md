# 🔧 Troubleshooting Guide

> **Common Issues and Solutions**

**Status:** 🟡 IN PROGRESS - Being updated as issues are discovered  
**Last Updated:** January 25, 2026

---

## Overview

This guide covers common problems encountered when deploying and operating Fortify, along with their solutions.

---

## Deployment Issues

### 1. Tor Won't Start / Can't Connect to Tor Control Port

**Symptoms:**
```
❌ Failed to connect to Tor control port: Connection refused
❌ Error: No such file: /var/lib/tor/control_auth_cookie
```

**Causes:**
- Tor service not running
- Wrong control port configuration
- Permission issues with cookie file

**Solutions:**

**Check Tor status:**
```bash
sudo systemctl status tor
```

**If not running, start it:**
```bash
sudo systemctl start tor
sudo systemctl enable tor  # Enable auto-start
```

**Verify control port is open:**
```bash
# Check Tor config
sudo cat /etc/tor/torrc | grep ControlPort

# Should show:
# ControlPort 9051
```

**Fix cookie permissions:**
```bash
sudo chmod 640 /var/lib/tor/control_auth_cookie
sudo chown debian-tor:debian-tor /var/lib/tor/control_auth_cookie

# Add fortify user to debian-tor group
sudo usermod -a -G debian-tor fortify
```

**Test connection manually:**
```bash
echo -e 'AUTHENTICATE ""\r\nGETINFO version\r\n' | nc 127.0.0.1 9051
```

---

### 2. Port Already in Use

**Symptoms:**
```
❌ Error binding to 0.0.0.0:8082: Address already in use
```

**Cause:**
Another service or previous Fortify instance using the port.

**Solution:**

**Find what's using the port:**
```bash
sudo netstat -tulpn | grep :8082
# or
sudo lsof -i :8082
```

**Kill the process:**
```bash
sudo kill -9 <PID>
```

**Or change Fortify's port:**
```toml
[http_proxy]
bind_address = "127.0.0.1:8083"  # Use different port
```

---

### 3. Missing Signing Key

**Symptoms:**
```
❌ Failed to load token signing key: No such file or directory
```

**Cause:**
Gate signing key not created.

**Solution:**

```bash
# Generate signing key
sudo mkdir -p /etc/fortify
sudo openssl rand -hex 32 > /etc/fortify/gate-signing.key
sudo chmod 600 /etc/fortify/gate-signing.key
sudo chown fortify:fortify /etc/fortify/gate-signing.key

# Verify it exists
ls -l /etc/fortify/gate-signing.key
```

---

### 4. Permission Denied Errors

**Symptoms:**
```
❌ Permission denied: /var/log/fortify/fortify.log
❌ Permission denied: /etc/fortify/fortify.toml
```

**Cause:**
Fortify running as non-root but files owned by root.

**Solution:**

```bash
# Create fortify user/group if doesn't exist
sudo useradd -r -s /bin/false fortify

# Fix ownership
sudo chown -R fortify:fortify /etc/fortify
sudo chown -R fortify:fortify /var/log/fortify
sudo chmod 700 /etc/fortify
sudo chmod 750 /var/log/fortify

# If systemd service, ensure User=fortify
sudo nano /etc/systemd/system/fortify-controller.service
```

---

## Runtime Issues

### 5. Backend Connection Timeouts

**Symptoms:**
```
❌ Backend request timeout after 60s
❌ Failed to connect to real service
⚠️ High error rate from nodes
```

**Causes:**
- Backend service down
- Wrong backend address configured
- Network issues
- Backend overwhelmed

**Solutions:**

**Test backend directly:**
```bash
# From Fortify server
torify curl http://your-real-service.onion/
```

**Check backend is listening:**
```bash
# On backend server
netstat -tulpn | grep :80
```

**Verify configuration:**
```toml
[service]
real_onion_address = "http://your-real-service.onion"  # Correct?
real_service_port = 80  # Correct?
```

**Increase timeout if backend is slow:**
```toml
[http_proxy]
connection_timeout_seconds = 60  # Increase if needed
```

---

### 6. CAPTCHAs Not Loading

**Symptoms:**
- Users see broken CAPTCHA images
- Blank captcha page
- "Failed to load challenge" error

**Causes:**
- Gate service not running
- CAPTCHA generation error
- Assets missing

**Solutions:**

**Check Gate is running:**
```bash
ps aux | grep fortify-gate
sudo netstat -tulpn | grep :8081
```

**Check logs for errors:**
```bash
sudo grep -i captcha /var/log/fortify/fortify.log | grep -i error
```

**Verify assets exist:**
```bash
ls -la /path/to/fortify/assets/html/captcha.html
ls -la /path/to/fortify/assets/images/captcha/
```

**Test Gate directly:**
```bash
curl http://127.0.0.1:8081/Fortify
```

---

### 7. High Memory Usage

**Symptoms:**
```
⚠️ Memory usage at 90%
System becoming slow
OOM killer triggered
```

**Causes:**
- Too many concurrent connections
- Memory leak
- Too many sessions cached

**Solutions:**

**Reduce connection limits:**
```toml
[http_proxy]
max_concurrent_connections = 500  # Lower
queue_size = 50  # Lower

[gate]
max_concurrent_verifications = 5  # Lower
```

**Restart to clear cache:**
```bash
sudo systemctl restart fortify-controller
```

**Monitor memory over time:**
```bash
# Watch memory usage
watch -n 5 'ps aux | grep fortify | awk "{sum+=\$6} END {print sum/1024 \" MB\"}"'
```

**Report potential leak:**
If memory grows continuously without bound, file issue with:
```bash
# Memory profile
ps aux | grep fortify
# Recent activity
sudo tail -100 /var/log/fortify/fortify.log
```

---

### 8. Sessions Not Promoting/Demoting

**Symptoms:**
- Users stuck at VERIFIED despite good behavior
- Attackers not getting demoted

**Causes:**
- Behavioral analysis disabled
- Thresholds too high
- Configuration issue

**Solutions:**

**Check behavioral analysis is enabled:**
```bash
# Should see behavioral analysis logs
sudo grep "behavioral\|violation" /var/log/fortify/fortify.log
```

**Review thresholds:**
```toml
# In fortify.toml (check these exist and are reasonable)
[trust]
promotion_threshold = 50  # Clean requests to promote
max_demotions_before_kill = 3  # Demotions before burn

[behavioral]
violation_type_thresholds = 3  # Violations before demotion
```

**Manually test demotion:**
```bash
# Use test script
./scripts/test-demotion.sh

# Or trigger manually
for i in {1..10}; do
    curl "http://your-mirror.onion/page$i" -H "Cookie: fortify_session=YOUR_TOKEN"
done
```

---

### 9. Mirrors Not Creating

**Symptoms:**
```
❌ Failed to create mirror
⚠️ No active mirrors available
```

**Causes:**
- Tor control connection failed
- Port exhaustion
- Tor service misconfigured

**Solutions:**

**Check Tor connectivity:**
```bash
echo -e 'AUTHENTICATE ""\r\nADD_ONION NEW:BEST Port=80,127.0.0.1:8082\r\n' | nc 127.0.0.1 9051
```

**Check Tor logs:**
```bash
sudo tail -f /var/log/tor/log
```

**Verify hidden service directory:**
```bash
sudo ls -la /var/lib/tor/fortify_mirrors/
sudo chown -R debian-tor:debian-tor /var/lib/tor/fortify_mirrors/
```

**Check Orchestrator logs:**
```bash
sudo grep -i "mirror\|orchestrator" /var/log/fortify/fortify.log | tail -20
```

---

### 10. Admin Panel Won't Load

**Symptoms:**
- 404 error on `/ctrl_xxx` path
- Login page not showing
- "Unauthorized" errors

**Causes:**
- Wrong admin path
- Not logged in
- Cookie issues

**Solutions:**

**Find correct admin path:**
```bash
# Check logs for admin panel URL
sudo grep "admin panel" /var/log/fortify/fortify.log
# Look for: ctrl_8f7k3m9x2n4p1q6w5v0b8c
```

**Clear cookies and retry:**
```
In Tor Browser:
1. Preferences → Privacy → Clear Data
2. Clear cookies only
3. Navigate to http://your-mirror.onion/ctrl_xxx/login
```

**Check password:**
```bash
# Default password (change in production!)
Password: pleaseletmein123
```

**Verify HTTP Proxy is running:**
```bash
ps aux | grep fortify-http
sudo netstat -tulpn | grep :8082
```

---

## Performance Issues

### 11. Slow Response Times

**Symptoms:**
- Pages load slowly
- Timeouts
- Poor user experience

**Causes:**
- Backend slow
- Too few nodes
- Resource exhaustion
- Network latency

**Solutions:**

**Check backend performance:**
```bash
# Time backend directly
time torify curl http://your-real-service.onion/
```

**Scale up nodes:**
```toml
[controller]
max_healthy_nodes = 10  # Increase
max_threat_nodes = 5
```

**Check system resources:**
```bash
top
free -h
iostat -x 1 5
```

**Optimize Tor:**
```toml
# In /etc/tor/torrc
CircuitBuildTimeout 30  # Faster circuit building
LearnCircuitBuildTimeout 0
```

---

### 12. High CPU Usage

**Symptoms:**
```
CPU constantly at 90-100%
System sluggish
```

**Causes:**
- Too many connections
- CAPTCHA generation overhead
- Attack in progress

**Solutions:**

**Identify which component:**
```bash
top -p $(pgrep -f fortify | tr '\n' ',' | sed 's/,$//')
```

**If Gate is high:**
```toml
[gate]
captcha_difficulty = "easy"  # Reduce complexity
max_concurrent_verifications = 5  # Limit parallel
```

**If HTTP Proxy is high:**
```toml
[http_proxy]
max_concurrent_connections = 300  # Reduce
```

**If nodes are high:**
```toml
[controller]
max_healthy_nodes = 15  # Distribute load
```

---

## Security Issues

### 13. Too Many Sessions Getting Burned

**Symptoms:**
- Many legitimate users getting burned
- High false positive rate

**Causes:**
- Thresholds too strict
- Overly aggressive behavioral analysis
- Attack patterns misconfigured

**Solutions:**

**Review burn rate:**
```bash
sudo grep "burned" /var/log/fortify/fortify.log | tail -50
```

**Increase tolerance:**
```toml
[trust]
max_demotions_before_kill = 5  # Was 3

[behavioral]
violation_type_thresholds = 5  # Was 3
```

**Disable specific checks:**
```bash
export BEHAVIORAL_PATH_ANALYSIS_ENABLED=false
export BEHAVIORAL_REFERER_ANALYSIS_ENABLED=false
```

**Review false positives:**
```bash
# What violations are triggering burns?
sudo grep "violation" /var/log/fortify/fortify.log | awk -F'(' '{print $2}' | awk -F')' '{print $1}' | sort | uniq -c
```

---

### 14. Attackers Not Being Blocked

**Symptoms:**
- Known bad actors still accessing
- Attack continues despite Fortify

**Causes:**
- Using CAPTCHA solving service
- Below detection thresholds
- Slow attack (under radar)

**Solutions:**

**Increase sensitivity:**
```toml
[behavioral]
violation_type_thresholds = 2  # Lower threshold

[trust]
max_demotions_before_kill = 2  # Faster burn
```

**Enable stricter CAPTCHAs:**
```toml
[gate]
captcha_difficulty = "hard"
```

**Review attack patterns:**
```bash
sudo grep "violation" /var/log/fortify/fortify.log | grep "Session.*xyz" | tail -50
# Replace xyz with attacker's session prefix
```

---

## Network Issues

### 15. Tor Circuit Issues

**Symptoms:**
```
⚠️ Circuits building slowly
⚠️ High circuit failure rate
```

**Causes:**
- Tor network congestion
- Vanguards configuration
- Local network issues

**Solutions:**

**Check Tor logs:**
```bash
sudo tail -f /var/log/tor/log
```

**Restart Tor:**
```bash
sudo systemctl restart tor
sleep 10  # Wait for circuits
sudo systemctl restart fortify-controller
```

**Adjust vanguards:**
```bash
# Temporarily disable to test
export VANGUARDS_ENABLED=false
sudo systemctl restart fortify-controller
```

**Check network connectivity:**
```bash
ping -c 5 1.1.1.1
curl -I https://check.torproject.org/
```

---

### 16. Mirror Address Changes

**Symptoms:**
- Old .onion addresses stop working
- Users can't find site

**Causes:**
- Mirrors rotated/burned
- Normal operation (mirrors are disposable)

**Solutions:**

**Get current active mirrors:**
```bash
sudo grep "Mirror.*ACTIVE" /var/log/fortify/fortify.log | tail -5
```

**Or use admin panel:**
```
http://your-mirror.onion/ctrl_xxx/mirrors
```

**Set up mirror discovery mechanism:**
> 🚧 **TODO:** Directory service/discovery mechanism not yet implemented.

**Workaround: Use directoryservice or keybase:**
```
Post current mirrors to:
- Your keybase account
- Trusted directory
- Social media (periodically)
```

---

## Data Issues

### 17. Session State Lost

**Symptoms:**
- Users forced to re-verify after restart
- Session counts reset

**Causes:**
- Fortify restart (sessions stored in memory)
- Normal behavior (by design)

**Solutions:**

This is **expected behavior**. Sessions are intentionally ephemeral.

If session continuity is needed:
> 🚧 **TODO:** Session continuity feature (Phase 4 of ROADMAP) not yet implemented.

Current workaround:
- Use longer session TTL
- Communicate to users that re-verification after server restart is normal

```toml
[gate]
token_lifetime_seconds = 7200  # 2 hours instead of 1
```

---

### 18. Logs Growing Too Large

**Symptoms:**
```
Disk space warning
/var/log/fortify/ using excessive space
```

**Cause:**
No log rotation configured.

**Solution:**

**Set up logrotate:**
```bash
sudo nano /etc/logrotate.d/fortify
```

```
/var/log/fortify/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 640 fortify fortify
}
```

**Manually clean old logs:**
```bash
sudo find /var/log/fortify/ -name "*.log.*" -mtime +7 -delete
```

**Reduce log verbosity:**
```toml
[logging]
level = "warn"  # Was "info" or "debug"
```

---

## Getting Help

### Diagnostic Information to Provide

When reporting issues, include:

```bash
# System info
uname -a
cat /etc/os-release

# Fortify version
git -C /path/to/fortify describe --tags

# Service status
sudo systemctl status fortify-controller tor

# Recent logs (sanitized!)
sudo tail -100 /var/log/fortify/fortify.log

# Configuration (remove sensitive data!)
sudo cat /etc/fortify/fortify.toml | grep -v "signing_key\|real_onion"

# Resource usage
free -h
df -h
top -bn1 | head -20
```

### Where to Get Help

1. **Check this guide** - Most common issues covered here
2. **Review logs** - Usually shows the problem
3. **Check documentation** - [Architecture](../01-Architecture/overview.md), [Configuration](../05-Configuration/configuration-reference.md)
4. **GitHub Issues** - Search existing issues or create new one
5. **Security issues** - Email security@fortify-project (TODO: create)

---

## Quick Reference

### Log Locations

| Log | Path |
|-----|------|
| Fortify | `/var/log/fortify/fortify.log` |
| Tor | `/var/log/tor/log` |
| Systemd | `journalctl -u fortify-controller` |

### Key Commands

```bash
# Restart Fortify
sudo systemctl restart fortify-controller

# Check status
sudo systemctl status fortify-controller tor

# View logs
sudo tail -f /var/log/fortify/fortify.log

# Test Tor
echo -e 'AUTHENTICATE ""\r\nGETINFO version\r\n' | nc 127.0.0.1 9051

# Check ports
sudo netstat -tulpn | grep fortify
```

---

> 🚧 **NOTE:** This guide is actively being updated as new issues are discovered during beta testing.  
> Last updated: January 25, 2026

*For operational procedures, see [Operations Guide](../06-Operations/monitoring.md)*
