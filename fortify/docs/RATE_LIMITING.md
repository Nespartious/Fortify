# Circuit-Based Rate Limiting System

## Overview

Fortify implements a **circuit-aware rate limiting system** to protect against DDoS attacks while ensuring legitimate users can always access the site, even during active attacks.

## The Problem

Traditional IP-based rate limiting fails for Tor hidden services because:
- All Tor users share the same IP address (`127.0.0.1` or "unknown")
- During a DDoS attack, the shared limit gets exhausted
- **Real users are blocked** before they can even attempt a CAPTCHA
- Attack volume: 1,500+ requests can block 9,400+ legitimate connection attempts

### Attack Analysis (January 19, 2026)

**Timeline:**
- 22:06-22:13: DDoS attack (1,500 malicious requests)
- **9,401 legitimate requests blocked** by shared IP rate limit
- Only 2 real users managed to access site (solved CAPTCHAs during brief gaps)
- Result: 99.98% of real users denied access

## The Solution: Circuit-Based Rate Limiting

### Architecture

#### **Layer 1: CAPTCHA Path Bypass**
```rust
// These paths NEVER trigger rate limiting
- /gate/captcha        (CAPTCHA challenge page)
- /gate/verify         (CAPTCHA submission)
- /Fortify/Portcullis  (rate limit info page with retry)
```

**Guarantee:** Real users can ALWAYS reach the CAPTCHA page, even during attacks.

#### **Layer 2: Per-Circuit Quotas**
Instead of tracking by IP, track by **circuit identifier**:

```rust
Circuit ID Priority:
1. Session token (if authenticated) → "session_abc123..."
2. Temporary fingerprint → "temp_{IP}_{UserAgent}"
```

**Per-Circuit Rate Limits:**
```
┌─────────────┬──────────────────┬──────────────────────┐
│ Trust Tier  │ Requests/10sec   │ Use Case             │
├─────────────┼──────────────────┼──────────────────────┤
│ Unknown     │ 10               │ New visitor          │
│ Verified    │ 100              │ Solved CAPTCHA       │
│ Trusted     │ 300              │ Long-term good actor │
└─────────────┴──────────────────┴──────────────────────┘
```

**Example:**
- Attack: 1,000 circuits each make 10 requests → all blocked after 10 req
- Real user: Makes 3 requests (page + CSS + JS) → **allowed** (independent quota)

#### **Layer 3: Attack Detection**
```rust
active_circuit_count() // Track unique circuits per time window

if active_circuits > 100 {
    // Probable DDoS - log but don't block (circuits already rate-limited)
    tracing::warn!("High circuit activity: {} circuits", count);
}
```

## How It Protects Real Users

### **Before (IP-based):**
```
Global Rate Limit: 75 requests/10sec for ALL Tor users

Attack Traffic:     [████████████████████] 1500 req → LIMIT HIT
Real User:          [X] Blocked before reaching CAPTCHA
Another Real User:  [X] Blocked before reaching CAPTCHA
```

**Result:** 9,401 legitimate users blocked

### **After (Circuit-based):**
```
Circuit A (Attack):   [██████████] 10 req → CIRCUIT A BLOCKED
Circuit B (Attack):   [██████████] 10 req → CIRCUIT B BLOCKED
Circuit C (Real User): [███] 3 req → ✓ ALLOWED (independent quota)
                       Solves CAPTCHA → 100 req/10s quota now
Circuit D (Real User): [█████] 5 req → ✓ ALLOWED (independent quota)
```

**Result:** Every real user gets their own quota, unaffected by attack traffic

## Real User Journey

### During Attack
```
1. User connects via Tor
   ├─ Request: GET /
   ├─ Circuit ID: temp_unknown_Mozilla/5.0...
   ├─ Quota: 10 requests/10sec (fresh circuit)
   └─ Result: ✓ Allowed (1/10 used)

2. Rate limited content redirect
   ├─ Request: GET / (attempts to load heavy content)
   ├─ Circuit quota: 10 req/10s used up
   └─ Redirect: /Fortify/Portcullis?reason=rate_limit

3. CAPTCHA challenge (NO RATE LIMIT)
   ├─ Request: GET /gate/captcha
   ├─ Rate limit: BYPASSED
   └─ Result: ✓ Show CAPTCHA (guaranteed access)

4. Solve CAPTCHA
   ├─ Request: POST /gate/verify
   ├─ Rate limit: BYPASSED
   ├─ CAPTCHA: Solved in 8 seconds
   └─ Result: Session token issued (Verified tier)

5. Access granted
   ├─ Request: GET /
   ├─ Circuit ID: session_f8a9c2...
   ├─ Quota: 100 requests/10sec (Verified tier)
   └─ Result: ✓ Full site access
```

### Attack Pattern Blocked
```
1. Attacker spawns 1000 circuits
   ├─ Each circuit: 10 requests to /
   └─ All circuits hit limit after 10 requests

2. Attacker tries to spam CAPTCHA endpoint
   ├─ Request: GET /gate/captcha (no rate limit)
   ├─ Cost: Must load full CAPTCHA image (~100KB)
   ├─ Cost: Must solve CAPTCHA (human time/bot compute)
   └─ Result: Expensive attack, limited impact

3. Attacker tries session cloning
   ├─ Stolen session makes 100 req/sec to /
   ├─ Circuit quota: 100 req/10s (Verified tier)
   ├─ Exceeded after 1 second
   └─ Result: Session rate limited, re-CAPTCHA required
```

## Implementation Details

### Circuit ID Generation

```rust
fn get_circuit_id(req: &Request, token_cookie: Option<&String>) -> String {
    if let Some(token_str) = token_cookie {
        // Authenticated user: use session ID
        if let Ok(token) = SessionToken::decode(token_str) {
            return format!("session_{}", &token_str[..16]);
        }
    }
    
    // Anonymous user: fingerprint from IP + User-Agent
    let ip = get_client_ip(req);
    let ua = req.headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    
    format!("temp_{}_{}", ip, &ua[..ua.len().min(20)])
}
```

### Rate Limit Check

```rust
fn handle_request(req: Request<Body>) -> Result<Response<Body>> {
    let path = req.uri().path();
    
    // Layer 1: Bypass for CAPTCHA paths
    if path.starts_with("/gate/") || path == "/Fortify/Portcullis" {
        return route_request(req); // Skip rate limiting
    }
    
    // Layer 2: Circuit-based rate limiting
    let circuit_id = get_circuit_id(&req, &token_cookie);
    let tier = get_trust_tier_from_token(&token_cookie);
    
    if !rate_limiter.check_and_record(&circuit_id, tier) {
        return redirect_to_gate_with_reason("rate_limit");
    }
    
    route_request(req)
}
```

## Behavioral Detection (Future Enhancement)

Additional signals to identify real users vs attackers:

### Real User Patterns
- **CAPTCHA solve time:** 3-30 seconds (human speed)
- **Asset loading:** Requests CSS, JS, images after page load
- **Navigation flow:** Homepage → Thread → Profile (logical browsing)
- **Timing gaps:** 1-60 seconds between pages (reading time)
- **Path diversity:** Visits 3+ different pages

### Attack Patterns  
- **CAPTCHA behavior:** Never attempts or instant solve (<1s = bot)
- **No assets:** Only requests HTML, never CSS/JS/images
- **Path spam:** 100+ requests to same path `/`
- **Timing:** <100ms between requests (automated)
- **No diversity:** Single path repeated

## Configuration

### Rate Limit Tuning
```toml
[rate_limiting]
# Per-circuit limits (requests per 10 seconds)
unknown_limit = 10      # New visitors (enough for CAPTCHA)
verified_limit = 100    # Solved CAPTCHA (normal browsing)
trusted_limit = 300     # Long-term users (power users)

# Bypass paths (never rate limited)
bypass_paths = [
    "/gate/",
    "/Fortify/Portcullis"
]

# Attack detection threshold
high_circuit_threshold = 100  # Log warning if exceeded
```

### Recommended Settings

**Low Traffic Site (<100 users/day):**
- Unknown: 10 req/10s
- Verified: 100 req/10s
- Trusted: 300 req/10s

**High Traffic Site (1000+ users/day):**
- Unknown: 15 req/10s
- Verified: 200 req/10s
- Trusted: 500 req/10s

**Under Active Attack:**
- Unknown: 5 req/10s (force CAPTCHA faster)
- Verified: 100 req/10s (protect verified users)
- Trusted: 300 req/10s (priority for known good users)

## Monitoring & Metrics

### Key Metrics to Track
```rust
// Per-minute aggregates
- unique_circuits_active     // Attack indicator (>100 = DDoS)
- rate_limited_circuits      // Blocked attackers
- bypassed_gate_requests     // CAPTCHA access (should never be blocked)
- verified_session_upgrades  // Real users solving CAPTCHAs
```

### Log Analysis
```bash
# Count rate limited circuits
grep "Rate limited circuit" fortify-http.log | wc -l

# See which circuits hit limits
grep "Rate limited circuit" fortify-http.log | awk '{print $5}' | sort | uniq -c

# Track CAPTCHA completions (real users)
grep "captcha verified.*captchas_remaining=0" fortify-gate.log | wc -l

# Identify session cloning attacks
grep "Rate limited circuit: session_" fortify-http.log
```

## Security Benefits

✅ **DDoS Mitigation:** Each attack circuit independently limited  
✅ **User Protection:** Real users always reach CAPTCHA page  
✅ **Session Cloning Defense:** Cloned sessions quickly hit per-circuit limits  
✅ **Adaptive Defense:** Higher limits for verified users  
✅ **Memory Efficient:** Circuit IDs automatically expire after 10 seconds  
✅ **Attack Visibility:** Track unique circuit count for threat intelligence

## Historical Context

### Attack: January 19, 2026
- **Before implementation:** 9,401 real users blocked by shared IP rate limit
- **After implementation:** Every real user reaches CAPTCHA, independent quotas
- **Attack volume:** 1,500 malicious requests over 7 minutes
- **Real users saved:** 2 users got through during attack → potentially 9,400+ with new system

## Testing

See [TESTING.md](../TESTING.md) for rate limiting test procedures.

### Test Scenarios

1. **Normal User Flow**
   - Visit homepage (1 request)
   - Load assets (3 requests)
   - Navigate to thread (1 request)
   - **Expected:** All requests allowed (5/10 quota used)

2. **Rate Limit Hit → CAPTCHA**
   - Make 10 requests rapidly to /
   - **Expected:** Redirected to /Fortify/Portcullis
   - Access /gate/captcha
   - **Expected:** CAPTCHA shown (no rate limit)

3. **CAPTCHA Bypass During Attack**
   - Simulate 100 attacking circuits
   - New user tries to access /gate/captcha
   - **Expected:** Instant access (bypassed rate limiting)

4. **Session Cloning Detection**
   - Valid session makes 100 requests in 1 second
   - **Expected:** Rate limited after 100 requests
   - Redirect to CAPTCHA for re-verification

## Future Enhancements

1. **Behavioral Scoring:** Track asset loading, timing patterns, path diversity
2. **CAPTCHA Difficulty Scaling:** Harder CAPTCHAs during high attack activity
3. **Temporary Allowlists:** Auto-trust circuits that complete multiple CAPTCHAs
4. **Circuit Reputation:** Long-term tracking of circuit behavior
5. **Adaptive Quotas:** Automatically adjust limits based on attack severity

---

**Last Updated:** January 19, 2026  
**Related Docs:** [SECURITY_AUDIT.md](SECURITY_AUDIT.md), [AUTHENTICATION.md](AUTHENTICATION.md)
