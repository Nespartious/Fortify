# Quick Test Commands Reference

Quick copy-paste commands for testing Fortify's defenses.

---

## Prerequisites

```bash
# Install required tools
sudo apt install curl jq apache2-utils

# Start Tor Browser or tor daemon
# Tor Browser: Use SOCKS on port 9150
# Tor daemon: Use SOCKS on port 9050
```

---

## Get Mirror Address

```bash
# Auto-detect from controller
MIRROR=$(curl -s http://127.0.0.1:8080/mirrors | jq -r '.mirrors[0].onion_address')
echo $MIRROR
```

---

## Run Test Scripts

```bash
# Automated stress test (no manual steps)
./scripts/stress-test.sh

# Interactive attack scenarios (requires CAPTCHA solving)
./scripts/test-attack-scenarios.sh

# Session demotion test (requires token)
./scripts/test-demotion.sh <mirror> <token>
```

---

## Manual Attack Tests

### Rate Limit Flood
```bash
# Send 50 rapid requests
for i in {1..50}; do 
    curl --socks5-hostname 127.0.0.1:9150 http://$MIRROR/ & 
done; wait
```

### Bot User-Agent
```bash
# Test bot detection
curl --socks5-hostname 127.0.0.1:9150 \
    -A "curl/7.68.0" \
    http://$MIRROR/
```

### Path Enumeration
```bash
# Enumerate common paths
for p in admin config api user login backup .env; do
    curl --socks5-hostname 127.0.0.1:9150 http://$MIRROR/$p
done
```

### Path Traversal
```bash
# Attempt directory traversal
curl --socks5-hostname 127.0.0.1:9150 \
    http://$MIRROR/../../../etc/passwd
```

### Sequential Paths
```bash
# Access sequential paths
for i in {1..10}; do
    curl --socks5-hostname 127.0.0.1:9150 http://$MIRROR/page$i
done
```

---

## Monitor System

### Watch Logs
```bash
# All logs
tail -f /var/log/fortify/fortify.log

# Violations only
tail -f /var/log/fortify/fortify.log | grep -E "violation|demotion|block"

# Rate limiting
tail -f /var/log/fortify/fortify.log | grep rate_limit
```

### Check Metrics
```bash
# Healthy node metrics
curl http://127.0.0.1:9100/metrics

# Violations count
curl http://127.0.0.1:9100/metrics | grep violation

# Rate limits
curl http://127.0.0.1:9100/metrics | grep rate_limit
```

### Check Sessions
```bash
# All sessions
curl http://127.0.0.1:8080/sessions | jq

# Count by tier
curl http://127.0.0.1:8080/sessions | jq '.sessions | group_by(.tier) | map({tier: .[0].tier, count: length})'

# Burned sessions
curl http://127.0.0.1:8080/sessions | jq '.sessions[] | select(.tier == -2)'
```

### System Health
```bash
# Controller health
curl http://127.0.0.1:8080/health

# Mirrors status
curl http://127.0.0.1:8080/mirrors | jq

# Node health
curl http://127.0.0.1:9100/health
curl http://127.0.0.1:9200/health
```

---

## Get Legitimate Token

### Via Tor Browser
```bash
# 1. Open Tor Browser
# 2. Visit http://yourmirror.onion/
# 3. Solve CAPTCHA
# 4. Press F12 > Storage > Cookies
# 5. Copy "fortify_session" value
TOKEN="paste_here"
```

### Test with Token
```bash
# Make authenticated request
curl --socks5-hostname 127.0.0.1:9150 \
    -b "fortify_session=$TOKEN" \
    http://$MIRROR/
```

---

## Advanced Tests

### Load Test with Apache Bench
```bash
# Through Tor (requires torsocks)
torsocks ab -n 1000 -c 10 http://$MIRROR/
```

### Distributed Attack (Multiple Circuits)
```bash
# Send NEWNYM to get new circuit
echo -e "AUTHENTICATE\nSIGNAL NEWNYM" | nc 127.0.0.1:9051
sleep 2

# Then make requests (will use new circuit)
curl --socks5-hostname 127.0.0.1:9150 http://$MIRROR/
```

### Slowloris Attack
```bash
# Clone slowloris
git clone https://github.com/gkbrk/slowloris.git
cd slowloris

# Attack through Tor
torsocks python3 slowloris.py $MIRROR -p 80 -s 200
```

---

## Verify Legitimate Access

### During Attack
```bash
# Terminal 1: Start attack
./scripts/stress-test.sh &

# Terminal 2: Test legitimate user (after 10s)
sleep 10
curl --socks5-hostname 127.0.0.1:9150 \
    -A "Mozilla/5.0 (Windows NT 10.0; rv:109.0) Gecko/20100101 Firefox/115.0" \
    -b "fortify_session=$LEGIT_TOKEN" \
    http://$MIRROR/

# Should get HTTP 200 or 302
```

---

## One-Liner Tests

```bash
# Count violations in last 100 log lines
tail -100 /var/log/fortify/fortify.log | grep -c violation

# Check if rate limiting is working
curl -s http://127.0.0.1:9100/metrics | grep rate_limit_exceeded

# Count active sessions
curl -s http://127.0.0.1:8080/sessions | jq '.sessions | length'

# Get blocked request count
curl -s http://127.0.0.1:9100/metrics | grep 'fortify_requests_total{status="403"}'

# Check system uptime
curl -s http://127.0.0.1:8080/health | jq .uptime_seconds
```

---

## Cleanup After Tests

```bash
# Clear test artifacts
rm -f /tmp/fortify_stress_results.txt

# Restart Fortify (clears sessions if not using persistence)
./target/release/fortify

# Or manually clear sessions via API (if implemented)
curl -X DELETE http://127.0.0.1:8080/sessions/clear
```

---

## Expected Results

### ✅ Good Signs
- Rate limits trigger after ~20 req/10sec
- Bot user-agents get HTTP 403
- Path traversal attempts blocked immediately
- Legitimate users get HTTP 200 during attacks
- System remains responsive under load
- Logs show violations being tracked

### ❌ Bad Signs
- No rate limiting (all requests succeed)
- Bot agents getting HTTP 200
- Path traversal reaches backend
- Legitimate users blocked during attacks
- System crashes or becomes unresponsive
- No violations in logs

---

## Quick Debugging

### Rate Limiting Not Working
```bash
# Check config
grep -A 5 rate_limit /etc/fortify/fortify.toml

# Verify requests hitting node
curl http://127.0.0.1:9100/metrics | grep requests_total
```

### Behavioral Analysis Not Detecting
```bash
# Check if enabled
grep behavioral_analysis /etc/fortify/fortify.toml

# Check thresholds
grep threshold /etc/fortify/fortify.toml

# Verify node is analyzing
tail -f /var/log/fortify/fortify.log | grep behavioral
```

### Sessions Not Being Created
```bash
# Check Gate is running
curl http://127.0.0.1:8081/health

# Check signing key exists
ls -la /etc/fortify/gate-signing.key

# Try manual captcha solve
curl http://127.0.0.1:8081/captcha
```

---

**For detailed testing procedures, see [tests/TESTING-GUIDE.md](TESTING-GUIDE.md)**
