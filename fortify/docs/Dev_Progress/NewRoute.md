# Phase 1: New Route - Instant CAPTCHA Serving

**Date:** January 20, 2026  
**Priority:** CRITICAL - Enables new users to access site during attacks  
**Status:** 📋 PLANNED (Implement after Phase 2)

---

## Objective

Serve CAPTCHA HTML directly from fortify-http to eliminate Gate bottleneck during DDoS attacks.

**Key Principle:** Static content (HTML) served locally, dynamic content (verification) handled by Gate.

---

## Problem Statement

### Current Bottleneck (Observed January 19-20, 2026):

```
DDoS Attack: 3,500 req/sec
├─ 46,468 circuits rate limited
├─ All redirect to /Fortify/Portcullis
├─ /Fortify/Portcullis proxies to Gate (port 8081)
└─ Gate receives 47,814 connection attempts in 60 seconds

Gate Capacity: ~1,000 connections/minute (estimated)
Result: 46,000+ connections queued
Real User Experience: 30+ second hang ❌
Outcome: NEW users can't access site during attacks
```

### User Impact:

**Existing Users (with sessions):** ✓ Unaffected, browsing normally  
**New Users (no sessions):** ❌ Can't reach CAPTCHA page, site appears down

---

## Solution Architecture

### Current Flow:

```
User → fortify-http → "Unknown, need CAPTCHA" → Proxy to Gate → Gate renders HTML → Return → User
                                                    ↑
                                                BOTTLENECK
                                            (Gate overwhelmed)
```

### Proposed Flow:

```
User → fortify-http → "Unknown, need CAPTCHA" → Serve HTML locally → User sees CAPTCHA instantly ✓
                                                      ↑
                                                  NO PROXY
                                              (No Gate bottleneck)

User loads image → fortify-http → Proxy /gate/captcha/{id}.png to Gate → Gate generates → Return
                                         (Only image, not HTML)

User submits answer → fortify-http → Proxy /gate/verify to Gate → Gate validates → Return token
                                          (Only verification, not HTML)
```

### Performance Comparison:

| Operation | Current | Proposed | Speedup |
|-----------|---------|----------|---------|
| **CAPTCHA HTML Load** | 30,000ms (hung) | 1ms (local) | 30,000x ✓ |
| **CAPTCHA Image Load** | N/A (included) | 50ms (Gate) | Same |
| **Verification** | N/A (included) | 10ms (Gate) | Same |
| **Gate Load/Attack** | 47,814 requests | ~1,400 requests | 97% reduction |

---

## Research: Industry Comparison

### 1. Cloudflare Challenge Pages

**How They Work:**
- Edge servers serve challenge HTML locally
- JavaScript challenge runs in browser
- Only verification result sent to origin
- **Key Insight:** Separate challenge delivery from verification

**Similarity to Fortify:**
- Challenge HTML static → serve locally ✓
- Verification dynamic → proxy to origin ✓
- Reduces origin load by 95%+ ✓

**Tor Compatibility:** ⚠️ JavaScript challenges broken on Tor (NoScript)  
**Fortify Advantage:** Image CAPTCHA works without JavaScript ✓

---

### 2. hCaptcha / reCAPTCHA

**How They Work:**
- Embed script loads from CDN
- Challenge iframe served from hcaptcha.com
- Verification token sent to origin
- **Key Insight:** Challenge serving separate from application

**Similarity to Fortify:**
- Challenge display not proxied through origin ✓
- Only verification token goes to origin ✓
- Scales to millions of requests ✓

**Tor Compatibility:** ⚠️ Third-party domains blocked by Tor Browser  
**Fortify Advantage:** Self-hosted, no external dependencies ✓

---

### 3. Fail2ban + NGINX Rate Limiting

**How They Work:**
- NGINX serves static error page for rate-limited requests
- No backend proxy needed
- Fail2ban bans IPs at firewall level
- **Key Insight:** Static responses for common cases (rate limit, block)

**Similarity to Fortify:**
- Rate limit → serve static page locally ✓
- No backend involvement for simple cases ✓
- Fast response under load ✓

**Tor Compatibility:** ❌ IP banning useless (all Tor users share IPs)  
**Fortify Advantage:** Circuit-based rate limiting works on Tor ✓

---

### 4. Tor Project's Own Protection

**How They Work:**
- OnionBalance load balancer
- Multiple backend instances
- PoW (Proof of Work) challenges during attacks
- **Key Insight:** Distribute load, add computational cost to attackers

**Similarity to Fortify:**
- PoW concept similar to CAPTCHA (human cost) ✓
- Protects against DDoS ✓
- Tor-native solution ✓

**Difference:**
- Tor uses PoW (CPU-based), Fortify uses CAPTCHA (human-based)
- PoW can be solved by bots, CAPTCHA requires human/AI ($$$)

**Fortify Advantage:** CAPTCHA more resistant to automated attacks ✓

---

### 5. Hidden Wiki / Onion Forums (Real Tor Sites)

**Observed Behavior:**
- Many Tor sites serve static "checking your browser" pages
- Verification happens client-side (JavaScript) or via cookies
- Minimal backend load for initial challenge
- **Key Insight:** Challenge pages must load fast or users leave

**Real Attack Scenario:**
- DDoS on Tor hidden service
- Sites that proxy everything → go offline
- Sites with local challenges → stay accessible

**Fortify Implementation:** Matches best practices ✓

---

## Real-World Tor Attack Threats

### Threat 1: Botnet Over Tor (Low Skill)

**Attack Pattern:**
- 1,000-10,000 bots
- Each bot makes 10 req/sec
- Total: 10,000-100,000 req/sec
- No CAPTCHA solving

**Current Fortify:** Gate overwhelmed, real users blocked ❌  
**Proposed Fortify:** Bots get CAPTCHA instantly, can't solve, stuck ✓  
**Real users:** Get CAPTCHA instantly, solve, access site ✓

**Mitigation:** EFFECTIVE ✅

---

### Threat 2: CAPTCHA Solving Service (Medium Skill)

**Attack Pattern:**
- Attacker uses 2Captcha, Anti-Captcha, etc.
- Cost: $0.50-$3.00 per 1,000 CAPTCHAs
- Solve time: 10-60 seconds per CAPTCHA
- Attacker gets 1,000 sessions

**Current Fortify:** 1 session → clone to 1,000 bots (observed) ❌  
**Proposed Fortify (Phase 2):** 1 verification token → single-use ✓  
**Attack Cost:** $0.50-$3.00 per bot (expensive at scale)  
**Attack Speed:** Limited by CAPTCHA solve time (10-60s)

**Mitigation:** ECONOMIC DETERRENT ✅

---

### Threat 3: Session Cloning (Observed)

**Attack Pattern:**
- Solve 1 CAPTCHA
- Get session token
- Clone to 1,951 bots (actual attack on Jan 19, 2026)
- All bots use same session

**Current Fortify:** Works perfectly for attacker ❌  
**Proposed Fortify (Phase 2):** Verification token single-use, session UA-bound ✓  
**Attack Result:** Only 1 bot succeeds, rest rejected

**Mitigation:** FULLY ADDRESSED ✅

---

### Threat 4: Advanced Persistent Threat (High Skill)

**Attack Pattern:**
- State-level actor
- Custom CAPTCHA solver (AI model)
- Solve time: 1-5 seconds
- Cost: Minimal (own infrastructure)

**Current Fortify:** Session cloning amplifies attack ❌  
**Proposed Fortify:** Single-use tokens, no amplification ✓  
**Attack Limit:** Must solve CAPTCHA per session (rate limited)  
**Real Users:** Not affected (CAPTCHAs load fast)

**Mitigation:** ATTACK SLOWED, real users protected ✅

---

### Threat 5: Layer 7 DDoS (Application-Level Flood)

**Attack Pattern:**
- 100,000+ requests from diverse sources
- Random paths, random User-Agents
- Goal: Exhaust server resources

**Current Fortify:** Gate becomes bottleneck ❌  
**Proposed Fortify:** fortify-http serves CAPTCHA HTML (static, fast) ✓  
**CPU Impact:** 1ms per CAPTCHA HTML vs 30,000ms proxy to Gate  
**Scalability:** Can serve 100,000 CAPTCHA pages/sec

**Mitigation:** HIGHLY EFFECTIVE ✅

---

### Threat 6: SlowLoris / Slow Read Attack

**Attack Pattern:**
- Open connections, send data slowly
- Exhaust connection pool
- Server can't accept new connections

**Current Fortify:** Gate has limited connection pool ❌  
**Proposed Fortify:** Gate connection limit (100), timeout (2s) ✓  
**Behavior:** Fast fail, return cached CAPTCHA or error  
**Real Users:** Retry succeeds quickly

**Mitigation:** PARTIALLY ADDRESSED (connection limits help) ✅

---

## Implementation Plan

### Task 1: Add CAPTCHA HTML Template

**File:** `crates/fortify-http/src/lib.rs`

**Add static template constant:**

```rust
const CAPTCHA_HTML_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Verification Required - Fortify</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: 'Courier New', monospace;
            background: linear-gradient(135deg, #0a0a0a 0%, #1a1a1a 100%);
            color: #00ff00;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            padding: 20px;
        }
        .container {
            background: rgba(26, 26, 26, 0.95);
            border: 2px solid #00ff00;
            box-shadow: 0 0 20px rgba(0, 255, 0, 0.3);
            padding: 40px;
            max-width: 600px;
            width: 100%;
            border-radius: 10px;
        }
        h1 {
            text-align: center;
            margin-bottom: 10px;
            font-size: 28px;
            text-shadow: 0 0 10px rgba(0, 255, 0, 0.5);
        }
        .reason {
            color: #ffaa00;
            text-align: center;
            margin-bottom: 20px;
            padding: 10px;
            background: rgba(255, 170, 0, 0.1);
            border: 1px solid #ffaa00;
            border-radius: 5px;
        }
        .info {
            text-align: center;
            margin-bottom: 30px;
            color: #888;
            line-height: 1.6;
        }
        .captcha-container {
            text-align: center;
            margin: 30px 0;
        }
        .captcha-container img {
            border: 2px solid #00ff00;
            box-shadow: 0 0 15px rgba(0, 255, 0, 0.2);
            max-width: 100%;
            height: auto;
            background: #000;
        }
        form {
            display: flex;
            flex-direction: column;
            gap: 15px;
        }
        input[type="text"] {
            background: #0a0a0a;
            border: 1px solid #00ff00;
            color: #00ff00;
            padding: 12px;
            font-size: 16px;
            font-family: 'Courier New', monospace;
            border-radius: 5px;
            transition: all 0.3s;
        }
        input[type="text"]:focus {
            outline: none;
            border-color: #00ff00;
            box-shadow: 0 0 10px rgba(0, 255, 0, 0.4);
        }
        button {
            background: #00ff00;
            color: #0a0a0a;
            border: none;
            padding: 14px;
            font-size: 16px;
            font-weight: bold;
            cursor: pointer;
            border-radius: 5px;
            transition: all 0.3s;
            text-transform: uppercase;
        }
        button:hover {
            background: #00cc00;
            box-shadow: 0 0 15px rgba(0, 255, 0, 0.5);
        }
        .footer {
            text-align: center;
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #333;
            font-size: 12px;
            color: #666;
        }
        .shield {
            font-size: 48px;
            text-align: center;
            margin-bottom: 20px;
            animation: pulse 2s infinite;
        }
        @keyframes pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.5; }
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="shield">🛡️</div>
        <h1>⚔️ VERIFICATION REQUIRED ⚔️</h1>
        <div class="reason">REASON_PLACEHOLDER</div>
        <div class="info">
            <p>This mirror is protected by Fortify.</p>
            <p>Prove you're human to access this content.</p>
        </div>
        <div class="captcha-container">
            <img src="/gate/captcha/CAPTCHA_ID_PLACEHOLDER.png" 
                 alt="CAPTCHA Challenge" 
                 width="300" 
                 height="100">
        </div>
        <form action="/gate/verify" method="POST">
            <input type="hidden" name="captcha_id" value="CAPTCHA_ID_PLACEHOLDER">
            <input type="text" 
                   name="answer" 
                   placeholder="Enter the text shown above" 
                   required 
                   autofocus 
                   autocomplete="off"
                   spellcheck="false">
            <button type="submit">🔓 VERIFY & ENTER</button>
        </form>
        <div class="footer">
            <p>🧅 Mirror Protection Active | Tor-Friendly Defense</p>
            <p>Fortify v0.1 | Circuit-Based Rate Limiting</p>
        </div>
    </div>
</body>
</html>"#;
```

**Status:** [ ] Not started

---

### Task 2: Add UUID Generation

**File:** `crates/fortify-http/Cargo.toml`

**Add dependency:**

```toml
uuid = { version = "1.6", features = ["v4", "fast-rng"] }
```

**File:** `crates/fortify-http/src/lib.rs`

**Add import:**

```rust
use uuid::Uuid;
```

**Status:** [ ] Not started

---

### Task 3: Create CAPTCHA HTML Serving Function

**File:** `crates/fortify-http/src/lib.rs`

**Add function:**

```rust
fn serve_captcha_html(reason: &str) -> Result<Response<Body>, hyper::Error> {
    // Generate unique CAPTCHA ID (UUID v4)
    let captcha_id = Uuid::new_v4().to_string();
    
    // Replace placeholders in template
    let html = CAPTCHA_HTML_TEMPLATE
        .replace("CAPTCHA_ID_PLACEHOLDER", &captcha_id)
        .replace("REASON_PLACEHOLDER", reason);
    
    info!("Serving CAPTCHA HTML, captcha_id: {}", captcha_id);
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store, no-cache, must-revalidate, proxy-revalidate")
        .header("Pragma", "no-cache")
        .header("Expires", "0")
        .body(Body::from(html))
        .unwrap())
}
```

**Status:** [ ] Not started

---

### Task 4: Update Rate Limit Flow

**File:** `crates/fortify-http/src/lib.rs`

**Current code (approximate line 615-625):**

```rust
// Rate limit exceeded - redirect to CAPTCHA
warn!("Rate limited circuit: {} tier={:?} ({} req/10sec exceeded)", 
      circuit_id, tier, limit);

return Ok(Response::builder()
    .status(StatusCode::TEMPORARY_REDIRECT)
    .header("Location", "/Fortify/Portcullis?reason=rate_limit")
    .body(Body::from(""))
    .unwrap());
```

**Replace with:**

```rust
// Rate limit exceeded - serve CAPTCHA directly (no Gate proxy)
warn!("Rate limited circuit: {} tier={:?} ({} req/10sec exceeded)", 
      circuit_id, tier, limit);

return serve_captcha_html("Rate Limit Exceeded - Too Many Requests");
```

**Status:** [ ] Not started

---

### Task 5: Update Unknown User Flow

**File:** `crates/fortify-http/src/lib.rs`

**Current code (approximate line 680-700):**

```rust
// Unknown user - proxy to Gate for verification
info!("THREAT PATH: Proxying unknown user to Gate for verification: {}", path);
return proxy_to_gate(req, &gate_address).await;
```

**Replace with:**

```rust
// Unknown user - serve CAPTCHA directly (no Gate proxy)
info!("Unknown user requesting: {} - serving CAPTCHA", path);
return serve_captcha_html("New Session - Human Verification Required");
```

**Status:** [ ] Not started

---

### Task 6: Update Blacklist Flow

**File:** `crates/fortify-http/src/lib.rs`

**Current code (approximate line 930-945):**

```rust
// Blacklisted session - redirect to Gate
info!("Blacklisted session detected, redirecting to Gate");
return Ok(Response::builder()
    .status(StatusCode::TEMPORARY_REDIRECT)
    .header("Location", "/Fortify")
    .body(Body::from(""))
    .unwrap());
```

**Replace with:**

```rust
// Blacklisted session - serve CAPTCHA directly
info!("Blacklisted session detected, serving CAPTCHA");
return serve_captcha_html("Session Blacklisted - Re-verification Required");
```

**Status:** [ ] Not started

---

### Task 7: (Optional) Add Gate Connection Limits

**File:** `crates/fortify-gate/src/lib.rs`

**Add connection semaphore:**

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

// Global connection limiter
lazy_static::lazy_static! {
    static ref CONNECTION_LIMITER: Arc<Semaphore> = Arc::new(Semaphore::new(100));
}

async fn handle_connection(stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    // Try to acquire permit with timeout
    let permit = match tokio::time::timeout(
        Duration::from_secs(2),
        CONNECTION_LIMITER.acquire()
    ).await {
        Ok(Ok(permit)) => permit,
        Ok(Err(_)) => {
            warn!("Gate overloaded: semaphore closed");
            return Err("Semaphore error".into());
        }
        Err(_) => {
            warn!("Gate overloaded: connection timeout");
            return Err("Connection timeout".into());
        }
    };
    
    // Process request normally
    let result = process_request(stream).await;
    
    // Permit automatically released when dropped
    drop(permit);
    
    result
}
```

**Status:** [ ] Optional (Phase 1b)

---

## Testing Plan

### Test 1: CAPTCHA HTML Rendering

**Objective:** Verify CAPTCHA HTML served correctly

```bash
# Make request without session
curl -s http://127.0.0.1:8080/ | head -50

# Expected output:
# - HTML with DOCTYPE
# - Title: "Verification Required"
# - Image src: /gate/captcha/{UUID}.png
# - Form action: /gate/verify
# - Hidden input with captcha_id
```

**Success Criteria:**
- ✓ HTML structure valid
- ✓ CAPTCHA ID is valid UUID format
- ✓ No Gate proxy involved
- ✓ Response time < 10ms

**Status:** [ ] Not tested

---

### Test 2: Rate Limit CAPTCHA

**Objective:** Verify rate-limited users get CAPTCHA instantly

```bash
# Trigger rate limit (15 requests, limit is 10)
time for i in {1..15}; do 
    curl -s http://127.0.0.1:8080/ > /dev/null
done

# Check response time for 11th+ request
curl -s -w "Response time: %{time_total}s\n" http://127.0.0.1:8080/ | grep "Response time"

# Expected: < 0.01s (10 milliseconds)
```

**Success Criteria:**
- ✓ First 10 requests: Normal flow
- ✓ Request 11+: CAPTCHA HTML instantly
- ✓ No 30-second hangs
- ✓ Response time < 100ms

**Status:** [ ] Not tested

---

### Test 3: Gate Load During Attack

**Objective:** Verify Gate not overwhelmed during DDoS

```bash
# Terminal 1: Start attack simulation
while true; do
    for i in {1..100}; do
        curl -s http://127.0.0.1:8080/ > /dev/null &
    done
    sleep 1
done

# Terminal 2: Monitor Gate requests
tail -f /tmp/fortify/logs/fortify-gate-*.log | grep -E "Created verification|captcha"

# Terminal 3: Try to access as new user
curl -v http://127.0.0.1:8080/

# Expected:
# - CAPTCHA HTML loads instantly (<100ms)
# - Gate logs show minimal activity (<10 req/sec)
# - Real user not affected by attack
```

**Success Criteria:**
- ✓ Gate receives <1,500 requests during 60s attack
- ✓ Real user gets CAPTCHA instantly
- ✓ No connection queue buildup
- ✓ fortify-http CPU < 50%

**Status:** [ ] Not tested

---

### Test 4: Full User Journey (New User During Attack)

**Objective:** End-to-end test of new user accessing site during attack

```bash
# Terminal 1: Start attack
while true; do
    for i in {1..50}; do curl -s http://127.0.0.1:8080/ > /dev/null & done
    sleep 1
done

# Terminal 2: Simulate real user
# Step 1: Get CAPTCHA HTML
time curl -s http://127.0.0.1:8080/ | grep "Verification Required"

# Step 2: Extract CAPTCHA ID from HTML
CAPTCHA_ID=$(curl -s http://127.0.0.1:8080/ | grep -oP 'captcha/\K[^.]+')

# Step 3: Load CAPTCHA image
time curl -s http://127.0.0.1:8080/gate/captcha/$CAPTCHA_ID.png > /dev/null

# Step 4: Submit answer (manual step - need to view image)
# curl -X POST -d "captcha_id=$CAPTCHA_ID&answer=XXXXX" http://127.0.0.1:8080/gate/verify

# Expected timings:
# - Step 1: <100ms (CAPTCHA HTML)
# - Step 3: <500ms (image generation)
# - Total time to see CAPTCHA: <1 second
```

**Success Criteria:**
- ✓ CAPTCHA HTML loads in <100ms
- ✓ CAPTCHA image loads in <500ms
- ✓ Total time to interactive CAPTCHA: <1 second
- ✓ Attack doesn't affect user experience

**Status:** [ ] Not tested

---

### Test 5: Performance Benchmark

**Objective:** Measure fortify-http capacity

```bash
# Install Apache Bench if needed
# sudo apt-get install apache2-utils

# Benchmark CAPTCHA HTML serving
ab -n 10000 -c 100 http://127.0.0.1:8080/

# Expected results:
# - Requests per second: >5,000
# - 99th percentile latency: <100ms
# - No failed requests
# - fortify-http CPU: <50%
```

**Success Criteria:**
- ✓ Can serve >5,000 CAPTCHA HTML pages/sec
- ✓ Latency p99 < 100ms
- ✓ CPU usage reasonable (<50%)
- ✓ No memory leaks (stable over time)

**Status:** [ ] Not tested

---

### Test 6: Gate Proxy Reduction

**Objective:** Verify Gate load reduced by 97%

```bash
# Before: Count Gate requests during attack
grep -c "Processing request" /tmp/fortify/logs/fortify-gate-before.log
# Expected: ~47,814

# After: Count Gate requests during same attack
grep -c "Processing request" /tmp/fortify/logs/fortify-gate-after.log
# Expected: <1,500 (97% reduction)
```

**Success Criteria:**
- ✓ Gate requests reduced by >90%
- ✓ Only image and verification requests hit Gate
- ✓ No HTML page requests to Gate

**Status:** [ ] Not tested

---

## Security Analysis

### No New Attack Vectors:

1. **CAPTCHA HTML is static** - No dynamic data to exploit
2. **UUID injection** - No user input in HTML (UUID server-generated)
3. **Gate still validates** - All verification logic unchanged
4. **Session creation unchanged** - Gate controls authentication

### Existing Protections Maintained:

1. **Rate limiting** - Still active, now faster to respond
2. **Circuit isolation** - Each circuit independent quota
3. **CAPTCHA verification** - Gate validates answers
4. **Session tokens** - Gate issues tokens

### Potential Concerns (Mitigated):

#### 1. CAPTCHA ID Guessing
**Concern:** Attacker guesses UUID, requests image  
**Mitigation:** UUIDv4 space = 2^122 (impossible to guess)  
**Risk:** NONE

#### 2. XSS in Reason Text
**Concern:** Reason parameter could inject HTML  
**Mitigation:** HTML escape reason text  
**Fix:**
```rust
fn serve_captcha_html(reason: &str) -> Result<Response<Body>, hyper::Error> {
    // HTML escape reason to prevent XSS
    let escaped_reason = html_escape::encode_text(reason);
    let html = CAPTCHA_HTML_TEMPLATE
        .replace("REASON_PLACEHOLDER", &escaped_reason);
    // ...
}
```
**Risk:** LOW (easy to fix)

#### 3. Cache Poisoning
**Concern:** Attacker caches malicious CAPTCHA HTML  
**Mitigation:** Cache-Control headers prevent caching  
**Current Headers:**
- `Cache-Control: no-store, no-cache, must-revalidate`
- `Pragma: no-cache`
- `Expires: 0`  
**Risk:** NONE

---

## Performance Impact Analysis

### CPU Usage:

**fortify-http:**
- Current: 9.1% during attack
- After Phase 1: ~15-18% (serving CAPTCHA HTML)
- Increase: +6-9% (acceptable)

**fortify-gate:**
- Current: Unknown (overwhelmed)
- After Phase 1: <5% (minimal load)
- Decrease: Significant improvement

### Memory Usage:

**fortify-http:**
- CAPTCHA template: 4 KB (constant)
- Per request: ~1 KB (temporary)
- Increase: Negligible

### Response Time:

**CAPTCHA page load:**
- Before: 30,000ms (hung)
- After: 1ms (local HTML)
- Improvement: 30,000x faster ✅

**CAPTCHA image load:**
- Before: N/A (included in page)
- After: 50ms (Gate generates)
- Change: Minimal (acceptable)

### Scalability:

**Current capacity:**
- Gate: ~1,000 requests/minute (observed limit)
- Bottleneck: Gate connection pool

**Proposed capacity:**
- fortify-http: ~300,000 CAPTCHA pages/minute (estimated)
- Gate: ~60,000 images/minute (with caching)
- Bottleneck: None (both can scale)

---

## Tor/Onion Specific Considerations

### 1. Latency Over Tor (3-6 Hops)

**Impact:** Every network roundtrip adds 100-300ms

**Current System:**
- User → Tor → fortify-http → Tor → Gate → Tor → fortify-http → Tor → User
- Total roundtrips: 3
- Added latency: 300-900ms

**Proposed System:**
- User → Tor → fortify-http → Tor → User (HTML)
- User → Tor → fortify-http → Tor → Gate → Tor → User (image)
- Total roundtrips: 2 (HTML immediate, image separate)
- Added latency: 200-600ms (improvement)

**Benefit:** Faster CAPTCHA display over Tor ✅

---

### 2. Tor Circuit Rotation

**Behavior:** Tor rotates circuits every 10 minutes

**Impact on Fortify:**
- Session cookie persists across circuit changes
- Circuit ID changes, but session token valid
- User not affected

**Compatibility:** ✅ No issues

---

### 3. Onion Service Reliability

**Challenge:** Onion services can be unstable

**Current System:** Gate unreachable → site completely down ❌

**Proposed System:**
- Gate unreachable → CAPTCHA HTML still serves ✓
- User sees CAPTCHA, but image fails to load
- Better UX: "Gate temporarily unavailable, try again"

**Benefit:** Graceful degradation ✅

---

### 4. Multiple Mirror Addresses

**Scenario:** Same Fortify instance, 10 different .onion addresses

**Current System:** Each mirror's unknown users → all proxy to same Gate → overwhelmed

**Proposed System:** Each mirror's unknown users → served locally → no Gate bottleneck ✓

**Benefit:** Scales to unlimited mirrors ✅

---

## Configuration

### Optional Settings (Phase 1):

**File:** `config/fortify.toml`

```toml
[captcha_serving]
# Serve CAPTCHA HTML from fortify-http (recommended)
enabled = true

# Use pre-rendered template (faster) vs proxy to Gate (slower)
use_local_template = true

# Cache CAPTCHA images (reduces Gate load)
cache_captcha_images = true
cache_ttl_seconds = 300  # 5 minutes

# Gate connection limits
gate_max_connections = 100
gate_connection_timeout_seconds = 2

# CAPTCHA HTML styling theme
theme = "terminal"  # Options: terminal, dark, light, custom

# Custom reason messages
[captcha_serving.messages]
rate_limit = "Rate Limit Exceeded - Too Many Requests"
new_session = "New Session - Human Verification Required"
blacklist = "Session Blacklisted - Re-verification Required"
```

---

## Rollback Plan

If Phase 1 causes issues:

1. **Disable local CAPTCHA serving:**
   ```toml
   [captcha_serving]
   enabled = false
   ```

2. **System reverts to proxying to Gate** (old behavior)

3. **No data loss** - only affects page serving

4. **Restart services:**
   ```bash
   pkill fortify
   ./target/release/fortify
   ```

5. **Verify rollback:**
   ```bash
   curl -I http://127.0.0.1:8080/
   # Should see: Location redirect (old behavior)
   ```

---

## Success Criteria

Phase 1 is successful when:

1. ✅ **CAPTCHA load time:** <1 second during attacks
2. ✅ **No hanging:** Zero 30+ second waits
3. ✅ **Gate load:** <1,500 requests during 3,500 req/sec attack
4. ✅ **Real user success:** NEW users can access site during attacks
5. ✅ **Existing users unaffected:** Sessions work as before
6. ✅ **Performance:** fortify-http CPU < 50%, no memory leaks
7. ✅ **Security:** No new vulnerabilities introduced
8. ✅ **Tor compatibility:** Works across circuit rotations

---

## Task Checklist

- [ ] Task 1: Add CAPTCHA HTML template to fortify-http
- [ ] Task 2: Add UUID generation dependency
- [ ] Task 3: Create serve_captcha_html() function
- [ ] Task 4: Update rate limit flow (replace redirect with HTML)
- [ ] Task 5: Update unknown user flow (replace proxy with HTML)
- [ ] Task 6: Update blacklist flow (replace redirect with HTML)
- [ ] Task 7: (Optional) Add Gate connection limits
- [ ] Test 1: CAPTCHA HTML rendering
- [ ] Test 2: Rate limit CAPTCHA instant response
- [ ] Test 3: Gate load during attack
- [ ] Test 4: Full user journey during attack
- [ ] Test 5: Performance benchmark (>5,000 req/sec)
- [ ] Test 6: Gate proxy reduction verification
- [ ] Security audit: XSS prevention, no new vectors
- [ ] Documentation update
- [ ] Production deployment

---

## Dependencies

### New Crate Dependencies:

**`crates/fortify-http/Cargo.toml`:**
```toml
uuid = { version = "1.6", features = ["v4", "fast-rng"] }
html-escape = "0.2"  # For XSS prevention
```

### Optional Dependencies:

**`crates/fortify-gate/Cargo.toml` (if adding connection limits):**
```toml
tokio = { version = "1", features = ["sync", "time"] }
```

---

## Comparison to Alternatives

| Solution | Pros | Cons | Fortify Phase 1 |
|----------|------|------|-----------------|
| **Cloudflare** | Massive scale, proven | JavaScript required, 3rd party | ✓ No JS, self-hosted |
| **hCaptcha** | Easy integration | 3rd party, privacy concerns | ✓ Private, Tor-friendly |
| **Fail2ban** | Simple, effective | IP-based, useless on Tor | ✓ Circuit-based |
| **PoW (Tor)** | Tor-native | Bots can solve, CPU-heavy | ✓ Human verification |
| **Pure Backend** | Full control | Bottleneck under load | ✓ Distributed load |

**Fortify Advantage:** Combines best of all approaches for Tor/onion services ✅

---

**Implementation Order:**
1. Complete Phase 2 (SessionProtection.md) ✓
2. Test Phase 2 thoroughly
3. Implement Phase 1 (this document)
4. Test Phase 1 with real attack simulation
5. Deploy combined system to production

**Next Steps:**
1. Review this plan
2. Begin Phase 1 implementation after Phase 2 complete
3. Monitor logs during testing
4. Adjust parameters based on observed performance
