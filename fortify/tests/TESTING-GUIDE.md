# Attack Simulation and Testing Guide

**Purpose:** Test Fortify's defensive capabilities by simulating real attacks and verifying legitimate users can still access the service.

---

## Available Testing Scripts

### 1. `test-attack-scenarios.sh` - Interactive Attack Simulator
Comprehensive attack testing with manual token collection.

**Features:**
- Rate limiting flood
- Path enumeration detection
- Bot user-agent blocking
- Form submission flood
- Sequential path access patterns
- Legitimate user verification

**Usage:**
```bash
./scripts/test-attack-scenarios.sh [mirror_address]
```

The script will prompt you to:
1. Solve a CAPTCHA in Tor Browser
2. Extract the session token
3. Run all attack scenarios
4. Verify legitimate access still works

### 2. `stress-test.sh` - Automated Stress Test
Fully automated stress testing without manual intervention.

**Features:**
- Rapid fire from single circuit
- Bot user-agent attacks
- Path enumeration attempts
- Parallel attack simulation
- Automatic metrics reporting

**Usage:**
```bash
./scripts/stress-test.sh [mirror_address] [duration_seconds]
```

**Example:**
```bash
# Run 60-second stress test
./scripts/stress-test.sh abc123.onion 60

# Auto-detect mirror and run default test
./scripts/stress-test.sh
```

### 3. `test-demotion.sh` - Session Demotion Test
Tests the trust tier demotion system.

**Usage:**
```bash
# Get a token first by solving CAPTCHA
./scripts/test-demotion.sh <mirror> <token>
```

---

## Manual Testing Scenarios

### Scenario 1: Rate Limit Attack

**Objective:** Trigger rate limiting and verify blocking works.

**Steps:**
```bash
# Terminal 1: Monitor logs
tail -f /var/log/fortify/fortify.log | grep -E "rate_limit|violation"

# Terminal 2: Attack
for i in {1..50}; do
    curl --socks5-hostname 127.0.0.1:9150 http://yourmirror.onion/ &
done
wait
```

**Expected:**
- First 20 requests: Success (HTTP 200)
- Remaining 30: Rate limited (HTTP 429)
- Logs show "rate_limit_exceeded" violations
- Session tracked in violation count

---

### Scenario 2: Bot User-Agent Detection

**Objective:** Verify bot detection via user-agent analysis.

**Steps:**
```bash
# Test bot agents
curl --socks5-hostname 127.0.0.1:9150 \
    -A "curl/7.68.0" \
    http://yourmirror.onion/

curl --socks5-hostname 127.0.0.1:9150 \
    -A "python-requests/2.25.1" \
    http://yourmirror.onion/
```

**Expected:**
- HTTP 403 or 400 response
- Logs show "bot_user_agent" violation
- Request blocked before reaching backend

---

### Scenario 3: Path Enumeration

**Objective:** Trigger path enumeration detection.

**Steps:**
```bash
# Rapid enumeration of common paths
for path in admin config api user login backup .env wp-admin; do
    curl --socks5-hostname 127.0.0.1:9150 \
        http://yourmirror.onion/$path
done
```

**Expected:**
- After ~5 unique paths: Detection triggered
- HTTP 403 response
- Logs show "resource_enumeration" violation
- Session marked suspicious

---

### Scenario 4: Path Traversal Attack

**Objective:** Verify path traversal is blocked.

**Steps:**
```bash
# Attempt directory traversal
curl --socks5-hostname 127.0.0.1:9150 \
    http://yourmirror.onion/../../../etc/passwd

curl --socks5-hostname 127.0.0.1:9150 \
    http://yourmirror.onion/..%2f..%2fetc%2fpasswd
```

**Expected:**
- Immediate HTTP 400 or 403
- Logs show "attack_path" violation with HIGH severity
- Path never forwarded to backend

---

### Scenario 5: Legitimate User During Attack

**Objective:** Verify legitimate users can access during attacks.

**Steps:**
```bash
# Terminal 1: Launch attack
./scripts/stress-test.sh &

# Terminal 2: Legitimate user (wait 10 seconds after attack starts)
sleep 10
curl --socks5-hostname 127.0.0.1:9150 \
    -A "Mozilla/5.0 (Windows NT 10.0; rv:109.0) Gecko/20100101 Firefox/115.0" \
    -b "fortify_session=YOUR_VALID_TOKEN" \
    http://yourmirror.onion/
```

**Expected:**
- Attack traffic is rate-limited/blocked
- Legitimate user gets HTTP 200/302
- Legitimate user routed through healthy node
- Attack traffic isolated to threat node or blocked

---

## Metrics to Monitor

### Node Metrics
```bash
# Check healthy node
curl http://127.0.0.1:9100/metrics
```

**Key metrics:**
- `fortify_requests_total` - Total requests processed
- `fortify_violations_total{type="rate_limit"}` - Rate limit violations
- `fortify_violations_total{type="bot_user_agent"}` - Bot detections
- `fortify_violations_total{type="attack_path"}` - Attack path blocks
- `fortify_demotions_total` - Sessions demoted

### Controller API
```bash
# Check session status
curl http://127.0.0.1:8080/sessions

# Check mirror health
curl http://127.0.0.1:8080/mirrors
```

---

## Real Attack Simulation

### Using Apache Bench (ab)
```bash
# Install if needed
sudo apt install apache2-utils

# Run load test through Tor
# Note: ab doesn't support SOCKS, so use torsocks wrapper
torsocks ab -n 1000 -c 10 http://yourmirror.onion/
```

### Using Siege
```bash
# Install
sudo apt install siege

# Configure for Tor
echo "proxy-host = 127.0.0.1" >> ~/.siege/siege.conf
echo "proxy-port = 9150" >> ~/.siege/siege.conf

# Run siege
siege -c 20 -r 50 http://yourmirror.onion/
```

### Using slowloris (slow HTTP attack)
```bash
# Clone slowloris
git clone https://github.com/gkbrk/slowloris.git
cd slowloris

# Run through Tor
torsocks python3 slowloris.py yourmirror.onion -p 80 -s 200
```

**Expected:** Fortify's connection limits and timeouts should mitigate slowloris.

---

## Advanced: Distributed Attack Simulation

Simulate a distributed attack using multiple Tor circuits:

```bash
#!/bin/bash
# distributed-attack.sh

MIRROR="$1"
REQUESTS=100

# Launch from multiple circuits (new identity each time)
for i in $(seq 1 10); do
    (
        # Send NEWNYM signal to get new circuit
        echo -e "AUTHENTICATE\nSIGNAL NEWNYM" | nc 127.0.0.1 9051
        sleep 2
        
        # Attack from this circuit
        for j in $(seq 1 $REQUESTS); do
            curl -s --socks5-hostname 127.0.0.1:9150 \
                "http://$MIRROR/" > /dev/null
        done
    ) &
done

wait
echo "Distributed attack complete"
```

**Expected:** Circuit-based rate limiting handles each circuit independently, so each should get quota separately.

---

## Testing Checklist

Use this checklist to verify all defensive features:

### Basic Protections
- [ ] Rate limiting blocks rapid requests
- [ ] Bot user-agents are detected and blocked
- [ ] Path traversal attempts are blocked
- [ ] Unknown sessions are sent to Gate (CAPTCHA)
- [ ] Invalid tokens are rejected

### Behavioral Analysis
- [ ] Path enumeration detected after 60 unique paths/minute
- [ ] Sequential path access detected (page1, page2, page3...)
- [ ] Form submission flood detected (>10 POST/minute)
- [ ] Referer missing on multiple requests detected

### Trust Tiers
- [ ] New users start at UNKNOWN tier
- [ ] CAPTCHA verification promotes to VERIFIED
- [ ] Good behavior promotes VERIFIED → TRUSTED
- [ ] Violations demote sessions
- [ ] 3 demotions = session burned
- [ ] Burned sessions cannot re-authenticate

### Demotion System
- [ ] Single violation doesn't immediately demote
- [ ] Multiple violations trigger demotion
- [ ] High severity violations count more
- [ ] Demoted users redirected to re-verification
- [ ] Demotion count persists across re-verification
- [ ] 3rd demotion = permanent burn

### Isolation
- [ ] Healthy node serves VERIFIED/TRUSTED
- [ ] Threat node serves SUSPICIOUS
- [ ] Burned sessions completely blocked
- [ ] Backend never exposed to untrusted traffic

### Legitimate User Access
- [ ] Legitimate users can access during attacks
- [ ] CAPTCHA remains solvable under load
- [ ] Session tokens remain valid
- [ ] Verified users not affected by attack traffic

---

## Troubleshooting Test Issues

### "Connection refused"
```bash
# Check Tor is running
sudo systemctl status tor

# Check Fortify components
ps aux | grep fortify
```

### "No mirrors found"
```bash
# Check controller
curl http://127.0.0.1:8080/mirrors

# Check orchestrator logs
tail -f /var/log/fortify/fortify.log | grep orchestrator
```

### "Tests don't trigger violations"
```bash
# Check behavioral analysis is enabled
grep "behavioral_analysis" /etc/fortify/fortify.toml

# Check thresholds aren't too high
grep "threshold" /etc/fortify/fortify.toml
```

### "Legitimate users are blocked"
```bash
# Check session token is valid
curl http://127.0.0.1:8080/sessions | jq

# Verify user-agent is normal
# Use Tor Browser's user-agent string
```

---

## Example Test Session

Complete example testing workflow:

```bash
# 1. Start Fortify
./target/release/fortify

# 2. Get mirror address
MIRROR=$(curl -s http://127.0.0.1:8080/mirrors | jq -r '.mirrors[0].onion_address')
echo "Testing mirror: $MIRROR"

# 3. Run automated stress test
./scripts/stress-test.sh "$MIRROR"

# 4. Monitor results
curl http://127.0.0.1:9100/metrics | grep violation
curl http://127.0.0.1:8080/sessions | jq

# 5. Test legitimate access
# Open Tor Browser, solve CAPTCHA, browse normally

# 6. Run comprehensive attack scenarios
./scripts/test-attack-scenarios.sh "$MIRROR"

# 7. Review logs
tail -n 100 /var/log/fortify/fortify.log | grep -E "violation|demotion|block"

# 8. Check system health
curl http://127.0.0.1:8080/health
```

---

## Performance Benchmarks

Typical performance metrics you should see:

**Healthy System:**
- Request latency: < 100ms
- Throughput: 100+ req/sec
- CAPTCHA solve time: 2-5 seconds
- Session verification: < 10ms
- Token validation: < 5ms

**Under Attack:**
- Legitimate user latency: < 200ms (minimal impact)
- Attack traffic blocking: > 90% blocked
- System remains responsive
- No crash or degradation

---

## Safety Considerations

**Don't:**
- Run attacks against production systems you don't own
- Use real user tokens in test scripts
- Test without monitoring (could burn legitimate sessions)
- Run tests on low-resource systems (may cause OOM)

**Do:**
- Test on dedicated dev/staging instances
- Monitor logs during tests
- Have legitimate access method ready
- Test incremental attack intensity
- Verify system recovers after tests

---

## Next Steps

After testing:
1. Review logs for unexpected behavior
2. Tune rate limits if needed
3. Adjust behavioral analysis thresholds
4. Test mirror burning process
5. Verify backup/recovery procedures

See [Troubleshooting Guide](../docs/Fortify Documentation/07-Troubleshooting/common-issues.md) for issues.
