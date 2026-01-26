# 📊 Operations & Monitoring

> **Running and Monitoring Fortify in Production**

**Status:** 🟡 IN PROGRESS - Basic operational procedures documented  
**Last Updated:** January 25, 2026

---

## Overview

This guide covers day-to-day operations, monitoring, and maintenance of a Fortify deployment.

---

## Daily Operations

### Health Checks

**Quick Health Check:**
```bash
# Check all services are running
ps aux | grep fortify

# Check Tor status
sudo systemctl status tor

# Check recent logs
sudo tail -n 50 /var/log/fortify/fortify.log
```

**Expected Output:**
```
✓ Controller running (PID: xxxx)
✓ Orchestrator running (PID: yyyy)
✓ HTTP Proxy running (PID: zzzz)
✓ Gate running (PID: aaaa)
✓ 3 mirrors active
✓ 5 healthy nodes active
✓ 2 threat nodes active
```

### Monitor Active Mirrors

```bash
# Via logs
sudo grep "Mirror created" /var/log/fortify/fortify.log | tail -5

# Via TUI
./target/release/fortify
# Navigate to "Mirrors" tab
```

**Expected Status:**
```
Mirror abc123.onion - ACTIVE - Age: 2h - Traffic: 150 req/hr
Mirror def456.onion - ACTIVE - Age: 5h - Traffic: 230 req/hr
Mirror ghi789.onion - ACTIVE - Age: 8h - Traffic: 180 req/hr
```

### Check Session Statistics

Using the admin panel:
```
http://your-mirror.onion/ctrl_8f7k3m9x2n4p1q6w5v0b8c
```

**Key Metrics:**
- Total sessions: Active, Verified, Burned
- Violation rate: % of sessions with violations
- Demotion rate: Sessions demoted per hour
- CAPTCHA solve rate: % successful verifications

---

## Monitoring

### Log Monitoring

**Real-time monitoring:**
```bash
# Follow all Fortify logs
sudo tail -f /var/log/fortify/fortify.log

# Filter for errors
sudo tail -f /var/log/fortify/fortify.log | grep ERROR

# Filter for security events
sudo tail -f /var/log/fortify/fortify.log | grep "violation\|demot\|burn"
```

### Important Log Patterns

**Normal Operations:**
```
✅ Session abc123 promoted to VERIFIED
✅ Mirror def456.onion created
✅ Healthy node started on port 9100
```

**Security Events:**
```
🚫 Session xyz789 demoted: 3 violations (PathEnumeration)
🔥 Session bad123 burned: max demotions reached
🛡️ Vanguards detected potential guard discovery
```

**System Warnings:**
```
⚠️ High circuit activity: 150 circuits
⚠️ Mirror abc123.onion high compromise score: 0.65
⚠️ Queue at 80% capacity
```

**Errors Requiring Action:**
```
❌ Failed to connect to Tor control port
❌ Backend unreachable: connection timeout
❌ Mirror creation failed: address already in use
```

### System Metrics

> 🚧 **TODO:** Prometheus/Grafana integration planned but not yet implemented.  
> Current monitoring is log-based.

**Monitor these resources:**

```bash
# CPU usage
top -p $(pgrep -f fortify | tr '\n' ',' | sed 's/,$//')

# Memory usage
ps aux | grep fortify | awk '{sum+=$6} END {print "Total RSS: " sum/1024 " MB"}'

# Disk usage
df -h /var/log/fortify
du -sh /var/lib/tor/fortify_mirrors/

# Network connections
netstat -anp | grep fortify | wc -l
```

### Alert Thresholds

| Metric | Warning | Critical | Action |
|--------|---------|----------|--------|
| CPU usage | >70% | >90% | Scale up nodes |
| Memory usage | >80% | >95% | Investigate leaks, restart |
| Disk usage | >80% | >95% | Rotate logs, clean mirrors |
| Error rate | >1% | >5% | Check backend, Tor connectivity |
| Burned sessions | >10/hour | >50/hour | Possible attack, review rules |

---

## Maintenance Tasks

### Daily

**Check health and logs:**
```bash
#!/bin/bash
# daily-check.sh

echo "=== Fortify Daily Health Check ==="
date

# Service status
systemctl is-active fortify-controller || echo "❌ Controller down"
systemctl is-active tor || echo "❌ Tor down"

# Recent errors
echo ""
echo "Recent errors (last 24h):"
sudo grep ERROR /var/log/fortify/fortify.log | tail -10

# Session stats
echo ""
echo "Session activity (last 1h):"
sudo grep -c "Session.*created" /var/log/fortify/fortify.log

# Mirror count
echo ""
echo "Active mirrors:"
sudo grep "Mirror.*ACTIVE" /var/log/fortify/fortify.log | tail -5
```

### Weekly

**Rotate logs:**
```bash
# Manual log rotation
sudo logrotate /etc/logrotate.d/fortify

# Or configure automatic rotation
sudo nano /etc/logrotate.d/fortify
```

```
/var/log/fortify/*.log {
    weekly
    rotate 4
    compress
    delaycompress
    missingok
    notifempty
    create 640 fortify fortify
    postrotate
        systemctl reload fortify-controller
    endscript
}
```

**Review security events:**
```bash
# Demotions
sudo grep "demoted" /var/log/fortify/fortify.log.1 | wc -l

# Burns
sudo grep "burned" /var/log/fortify/fortify.log.1 | wc -l

# Vanguards alerts
sudo grep "Vanguards" /var/log/fortify/fortify.log.1
```

### Monthly

**Update Fortify:**
```bash
# Backup configuration
sudo tar -czf fortify-backup-$(date +%Y%m%d).tar.gz \
    /etc/fortify/ \
    /var/log/fortify/

# Pull updates
cd /path/to/fortify
git pull origin main

# Rebuild
cargo build --release --workspace

# Restart (during low-traffic window)
sudo systemctl restart fortify-controller
```

**Clean old mirrors:**
```bash
# List old mirror directories
sudo ls -lht /var/lib/tor/fortify_mirrors/ | tail -20

# Remove mirrors older than 30 days
sudo find /var/lib/tor/fortify_mirrors/ -type d -mtime +30 -exec rm -rf {} \;
```

**Review configuration:**
```bash
# Check for deprecated options
diff /etc/fortify/fortify.toml /path/to/fortify/config/fortify.example.toml

# Validate current config
./target/release/fortify-controller --config /etc/fortify/fortify.toml --check
```

---

## Common Operational Tasks

### Restart Services

**Graceful restart:**
```bash
# Drain connections first (wait for active requests to complete)
sudo systemctl reload fortify-controller

# Full restart
sudo systemctl restart fortify-controller
sudo systemctl restart tor
```

**Emergency restart:**
```bash
sudo systemctl stop fortify-controller
sleep 5
sudo systemctl start fortify-controller
```

### Manual Mirror Burn

Using admin panel:
```
1. Navigate to http://your-mirror.onion/ctrl_xxx/mirrors
2. Find compromised mirror
3. Click "Burn Mirror"
4. Confirm action
```

Or via API:
```bash
curl -X POST http://127.0.0.1:8080/mirror/burn \
  -H "X-Fortify-Admin-Token: YOUR_TOKEN" \
  -d '{"mirror_id": "abc123.onion"}'
```

### Force Mirror Rotation

```bash
# Burn all old mirrors
curl -X POST http://127.0.0.1:8080/mirror/rotate-all \
  -H "X-Fortify-Admin-Token: YOUR_TOKEN"

# System will auto-create replacements
```

### Clear Burned Sessions

> 🚧 **TODO:** API endpoint for bulk session cleanup not yet implemented.

Current workaround - restart to clear in-memory state:
```bash
sudo systemctl restart fortify-controller
```

### Adjust Behavioral Analysis Thresholds

Edit configuration and reload:
```bash
sudo nano /etc/fortify/fortify.toml

# Update thresholds
[behavioral]
violation_type_thresholds = 5  # Increase tolerance

sudo systemctl reload fortify-controller
```

Or use environment variables (temporary):
```bash
export BEHAVIORAL_VIOLATION_THRESHOLD=5
sudo systemctl restart fortify-controller
```

---

## Performance Tuning

### Identify Bottlenecks

**CPU-bound:**
```bash
# Check CPU per component
top -p $(pgrep -f fortify | tr '\n' ',' | sed 's/,$//')

# If Gate or Proxy at 100%, increase node count
```

**Memory-bound:**
```bash
# Check memory usage
free -h
ps aux --sort=-%mem | grep fortify | head -5

# If high, reduce max_concurrent_connections
```

**I/O-bound:**
```bash
# Check disk I/O
iostat -x 1 5

# If high, move logs to faster disk or reduce log level
```

### Optimization Strategies

**High CPU on Gate:**
```toml
[controller]
max_healthy_nodes = 15  # Distribute CAPTCHA load

[gate]
max_concurrent_verifications = 20  # Allow more parallel verifications
```

**High CPU on HTTP Proxy:**
```toml
[controller]
max_healthy_nodes = 10  # More nodes for routing

[http_proxy]
max_concurrent_connections = 1500  # Increase capacity
```

**High memory usage:**
```toml
[http_proxy]
queue_size = 50  # Reduce queue depth
max_concurrent_connections = 500  # Lower limit

[gate]
max_concurrent_verifications = 5  # Fewer parallel sessions
```

---

## Backup and Recovery

### What to Backup

**Critical:**
- `/etc/fortify/fortify.toml` - Configuration
- `/etc/fortify/*.key` - Signing keys

**Important:**
- `/var/log/fortify/` - Logs (for analysis)

**Optional:**
- `/var/lib/tor/fortify_mirrors/` - Mirror state (regenerated automatically)

### Backup Script

```bash
#!/bin/bash
# backup-fortify.sh

BACKUP_DIR="/var/backups/fortify"
DATE=$(date +%Y%m%d-%H%M%S)

mkdir -p $BACKUP_DIR

# Backup configuration
sudo tar -czf $BACKUP_DIR/fortify-config-$DATE.tar.gz \
    /etc/fortify/

# Backup logs (last 7 days)
sudo tar -czf $BACKUP_DIR/fortify-logs-$DATE.tar.gz \
    --newer $(date -d '7 days ago' +%Y-%m-%d) \
    /var/log/fortify/

# Keep only last 30 days of backups
find $BACKUP_DIR -type f -mtime +30 -delete

echo "Backup complete: $BACKUP_DIR"
```

### Recovery Procedure

**Restore configuration:**
```bash
# Extract backup
sudo tar -xzf fortify-config-YYYYMMDD.tar.gz -C /

# Verify permissions
sudo chown -R fortify:fortify /etc/fortify
sudo chmod 600 /etc/fortify/*.key

# Restart services
sudo systemctl restart fortify-controller
```

**Restore from scratch:**
```bash
# Reinstall Fortify
git clone https://github.com/your-org/fortify.git
cd fortify
cargo build --release --workspace

# Restore configuration
sudo cp backup/fortify.toml /etc/fortify/
sudo cp backup/*.key /etc/fortify/

# Restart
sudo systemctl restart fortify-controller
```

---

## Troubleshooting Operations

### Service Won't Start

```bash
# Check logs for errors
sudo journalctl -u fortify-controller -n 50

# Common issues:
# - Tor not running
# - Port already in use
# - Missing signing key
# - Invalid configuration
```

### High Error Rate

```bash
# Identify error types
sudo grep ERROR /var/log/fortify/fortify.log | awk '{print $5}' | sort | uniq -c

# Common errors:
# - Backend timeout: Check backend connectivity
# - Tor control error: Restart Tor
# - Token validation: Check signing key
```

### Mirrors Not Creating

```bash
# Check Tor control connection
echo -e 'AUTHENTICATE ""\r\nGETINFO version\r\n' | nc 127.0.0.1 9051

# Check permissions
sudo ls -l /var/lib/tor/control_auth_cookie

# Check logs
sudo tail -f /var/log/tor/log
```

---

## Security Operations

### Respond to Attack

**If seeing high attack volume:**

1. **Increase CAPTCHA difficulty**
   ```toml
   [gate]
   captcha_difficulty = "hard"
   ```

2. **Lower rate limits**
   ```bash
   export RATE_LIMIT_UNKNOWN_TIER=5
   export RATE_LIMIT_VERIFIED_TIER=50
   ```

3. **Burn compromised mirrors**
   ```bash
   # Via admin panel or API
   ```

4. **Monitor logs**
   ```bash
   sudo tail -f /var/log/fortify/fortify.log | grep "violation\|burn"
   ```

### Review Security Incidents

```bash
# Extract all security events from last 24h
sudo grep -E "demot|burn|violation" /var/log/fortify/fortify.log > security-$(date +%Y%m%d).log

# Count by violation type
grep "violation" security-$(date +%Y%m%d).log | awk -F'(' '{print $2}' | awk -F')' '{print $1}' | sort | uniq -c
```

---

## Next Steps

- Review [Troubleshooting Guide](../07-Troubleshooting/common-issues.md) for specific issues
- Configure [Behavioral Analysis](../02-Core-Concepts/behavioral-analysis.md) thresholds
- Set up external monitoring (Prometheus integration planned)

---

> 🚧 **TODO:** 
> - Prometheus metrics export
> - Grafana dashboard templates
> - Alert manager integration
> - Automated health check scripts

*For configuration details, see [Configuration Reference](../05-Configuration/configuration-reference.md)*
