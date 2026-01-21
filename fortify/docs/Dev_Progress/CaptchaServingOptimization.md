# CAPTCHA Serving Optimization - Gate Bottleneck Fix

**Date:** January 20, 2026  
**Issue:** Gate overwhelmed during DDoS attacks, real users hang waiting for CAPTCHA  
**Solution:** Serve CAPTCHA HTML from fortify-http, only proxy verification to Gate  
**Status:** 📋 PLANNED

---

## Problem Statement

### Current Behavior

During DDoS attack (3,500 req/sec):
- 46,468 circuits rate limited
- All redirected to `/Fortify/Portcullis`
- `/Fortify/Portcullis` proxies to fortify-gate (port 8081)
- Gate receives ~47,814 connections
- **Gate overwhelmed** → connection queue builds up
- **Real users hang 30+ seconds** waiting for CAPTCHA page
- Users think site is down ❌

### Root Cause

**Every CAPTCHA page view proxies to Gate** → Gate becomes single point of failure

**Gate Load During Attack:**
- 1,346 Unknown user proxies (before rate limit)
- 46,468 rate limit redirects (all proxy to Gate)
- Total: ~47,814 Gate connections in 60 seconds
- Gate can't keep up → users queue

---

## Solution Overview

### Core Changes

1. **fortify-http serves CAPTCHA HTML directly**
   - No Gate proxy for CAPTCHA page display
   - Ultra-fast response (<1ms vs 30+ seconds)
   - Static HTML template with dynamic UUID

2. **fortify-gate handles verification only**
   - CAPTCHA image generation: `/gate/captcha/{id}.png`
   - Answer verification: `/gate/verify`
   - Session token creation
   - 97% reduction in Gate load

3. **Gate connection limits**
   - Max 100 concurrent connections
   - 2-second timeout for overload
   - Fast fail instead of hanging

### Benefits

- **User Experience:** CAPTCHA loads instantly during attacks
- **Gate Protection:** 97% less load (47,814 → 1,400 requests)
- **Attack Resistance:** Attackers can't exhaust Gate connection pool
- **Resource Efficiency:** fortify-http handles static content (fast), Gate focuses on verification (crypto)

---

## Implementation Plan

### Phase 1: CAPTCHA HTML Template in fortify-http

**File:** `crates/fortify-http/src/lib.rs`

**Add static HTML template:**

```rust
// CAPTCHA HTML template - served directly from fortify-http
const CAPTCHA_HTML_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Verification Required - Fortify</title>
    <style>
        body { font-family: monospace; background: #0a0a0a; color: #00ff00; 
               display: flex; justify-content: center; align-items: center; 
               min-height: 100vh; margin: 0; }
        .container { background: #1a1a1a; border: 2px solid #00ff00; 
                     padding: 40px; max-width: 500px; text-align: center; }
        img { border: 2px solid #00ff00; margin: 20px 0; max-width: 100%; }
        input { background: #0a0a0a; border: 1px solid #00ff00; color: #00ff00; 
                padding: 10px; width: 100%; box-sizing: border-box; margin: 10px 0; }
        button { background: #00ff00; color: #0a0a0a; border: none; 
                 padding: 12px 30px; cursor: pointer; font-weight: bold; }
        button:hover { background: #00cc00; }
        .reason { color: #ffaa00; margin-bottom: 20px; }
    </style>
</head>
<body>
    <div class="container">
        <h1>⚔️ VERIFICATION REQUIRED</h1>
        <div class="reason">REASON_PLACEHOLDER</div>
        <p>Prove you're human to access this mirror.</p>
        <img src="/gate/captcha/CAPTCHA_ID_PLACEHOLDER.png" alt="CAPTCHA">
        <form action="/gate/verify" method="POST">
            <input type="hidden" name="captcha_id" value="CAPTCHA_ID_PLACEHOLDER">
            <input type="text" name="answer" placeholder="Enter the text above" 
                   required autofocus autocomplete="off">
            <button type="submit">Verify</button>
        </form>
        <p style="font-size: 12px; color: #666; margin-top: 20px;">
            Mirror Protection Active | Tor-Friendly Defense
        </p>
    </div>
</body>
</html>"#;
```

**Add UUID generation dependency:**

In `Cargo.toml`:
```toml
uuid = { version = "1.6", features = ["v4", "fast-rng"] }
```

**Add function to serve CAPTCHA HTML:**

```rust
use uuid::Uuid;

fn serve_captcha_html(reason: &str) -> Result<Response<Body>, hyper::Error> {
    // Generate unique CAPTCHA ID
    let captcha_id = Uuid::new_v4().to_string();
    
    // Replace placeholders in template
    let html = CAPTCHA_HTML_TEMPLATE
        .replace("CAPTCHA_ID_PLACEHOLDER", &captcha_id)
        .replace("REASON_PLACEHOLDER", reason);
    
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store, no-cache, must-revalidate")
        .body(Body::from(html))
        .unwrap())
}
```

---

### Phase 2: Update Rate Limit Flow

**File:** `crates/fortify-http/src/lib.rs`

**Current code (lines ~610-625):**

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

**New code:**

```rust
// Rate limit exceeded - serve CAPTCHA directly (no Gate proxy)
warn!("Rate limited circuit: {} tier={:?} ({} req/10sec exceeded)", 
      circuit_id, tier, limit);

return serve_captcha_html("Rate Limit Exceeded - Too many requests");
```

---

### Phase 3: Update Unknown User Flow

**File:** `crates/fortify-http/src/lib.rs`

**Current code (lines ~680-700):**

```rust
// Unknown user - proxy to Gate for verification
info!("THREAT PATH: Proxying unknown user to Gate for verification: {}", path);
return proxy_to_gate(req, &gate_address).await;
```

**New code:**

```rust
// Unknown user - serve CAPTCHA directly
info!("Unknown user requesting: {} - serving CAPTCHA", path);
return serve_captcha_html("New Session - Human Verification Required");
```

---

### Phase 4: Update Blacklist Flow

**File:** `crates/fortify-http/src/lib.rs`

**Current code (lines ~930-945):**

```rust
// Blacklisted session - redirect to Gate
info!("Blacklisted session detected, redirecting to Gate");
return Ok(Response::builder()
    .status(StatusCode::TEMPORARY_REDIRECT)
    .header("Location", "/Fortify")
    .body(Body::from(""))
    .unwrap());
```

**New code:**

```rust
// Blacklisted session - serve CAPTCHA directly
info!("Blacklisted session detected, serving CAPTCHA");
return serve_captcha_html("Session Blacklisted - Re-verification Required");
```

---

### Phase 5: Gate Connection Limits

**File:** `crates/fortify-gate/src/lib.rs`

**Add connection limiting (optional but recommended):**

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

// Global connection limiter
static CONNECTION_LIMITER: Lazy<Arc<Semaphore>> = Lazy::new(|| {
    Arc::new(Semaphore::new(100)) // Max 100 concurrent connections
});

async fn handle_connection(stream: TcpStream) {
    // Try to acquire permit (non-blocking with timeout)
    match tokio::time::timeout(
        Duration::from_secs(2),
        CONNECTION_LIMITER.acquire()
    ).await {
        Ok(Ok(permit)) => {
            // Process connection normally
            process_request(stream).await;
            drop(permit); // Release permit
        }
        Ok(Err(_)) | Err(_) => {
            // Semaphore closed or timeout - reject connection
            warn!("Gate overloaded, rejecting connection");
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }
}
```

---

## Testing Plan

### Test 1: Normal User Flow

**Scenario:** New user visits site with no session

**Expected Behavior:**
1. User requests `/` 
2. fortify-http detects no session
3. **CAPTCHA HTML served instantly** (no Gate proxy)
4. User sees CAPTCHA in <1 second ✓
5. User loads CAPTCHA image: `/gate/captcha/{id}.png`
6. Gate generates image, returns PNG
7. User submits answer to `/gate/verify`
8. Gate verifies answer, creates session token
9. User gets cookie, redirects to `/`
10. User accesses site normally

**Verification Commands:**
```bash
# Test CAPTCHA display
curl -v http://127.0.0.1:8080/ 2>&1 | grep -A5 "CAPTCHA"

# Should see HTML with embedded CAPTCHA image reference
# Should NOT see "Location: /Fortify/Portcullis" redirect
```

---

### Test 2: Rate Limit During Attack

**Scenario:** DDoS attack in progress, user exceeds 10 req/10s

**Expected Behavior:**
1. User sends 15 requests rapidly
2. First 10 requests: OK
3. Request 11+: Rate limited
4. **CAPTCHA HTML served instantly** (no hang)
5. User sees CAPTCHA in <1 second ✓
6. User solves CAPTCHA → upgraded to Verified tier (100 req/10s)

**Verification Commands:**
```bash
# Trigger rate limit
for i in {1..15}; do 
    curl -s -w "Response time: %{time_total}s\n" http://127.0.0.1:8080/ 
done

# Should see:
# - First 10: Normal responses or redirects
# - Last 5: CAPTCHA HTML (instant, <0.1s response time)
# - NO 30-second hangs ✓
```

---

### Test 3: Gate Load During Attack

**Scenario:** Simulate 3,500 req/sec DDoS

**Expected Behavior:**
- fortify-http: Handles all rate limit redirects (serves CAPTCHA HTML)
- fortify-gate: Only handles image requests + verifications
- Gate load: <100 requests (vs 47,814 before)
- Real user: Gets CAPTCHA instantly

**Verification Commands:**
```bash
# Monitor Gate requests during attack
tail -f /tmp/fortify/logs/fortify-gate-*.log | grep -E "Created verification|captcha"

# Count Gate requests (should be minimal)
grep -c "Created verification" /tmp/fortify/logs/fortify-gate-*.log

# Should see <100 requests during attack (vs 1,346+ before)
```

---

### Test 4: CAPTCHA HTML Rendering

**Scenario:** Verify CAPTCHA HTML is correct

**Expected Behavior:**
- HTML contains proper CAPTCHA ID (UUID format)
- Image src points to `/gate/captcha/{id}.png`
- Form action posts to `/gate/verify`
- Hidden field contains CAPTCHA ID

**Verification Commands:**
```bash
# Get CAPTCHA HTML
curl -s http://127.0.0.1:8080/ | grep -E "captcha|CAPTCHA"

# Should see:
# - <img src="/gate/captcha/[UUID].png">
# - <input name="captcha_id" value="[UUID]">
# - <form action="/gate/verify" method="POST">
```

---

### Test 5: Gate Connection Limit (if implemented)

**Scenario:** Flood Gate with 200 concurrent connections

**Expected Behavior:**
- First 100 connections: Accepted
- Connections 101+: Rejected with 2s timeout
- No indefinite hanging

**Verification Commands:**
```bash
# Flood Gate with connections
for i in {1..200}; do
    curl -s --max-time 3 http://127.0.0.1:8081/gate/captcha/test.png &
done
wait

# Check Gate logs for rejections
grep "overloaded" /tmp/fortify/logs/fortify-gate-*.log

# Should see ~100 rejections
```

---

## Performance Metrics

### Before Implementation:

| Metric | Value |
|--------|-------|
| **CAPTCHA page load time (during attack)** | 30+ seconds (hung) |
| **Gate requests per attack** | ~47,814 |
| **Real users blocked** | 100% (couldn't reach CAPTCHA) |
| **fortify-http CPU** | 9.1% |
| **fortify-gate CPU** | Unknown (overwhelmed) |

### After Implementation (Expected):

| Metric | Value |
|--------|-------|
| **CAPTCHA page load time (during attack)** | <1 second ✓ |
| **Gate requests per attack** | ~1,400 (97% reduction) |
| **Real users blocked** | 0% (instant CAPTCHA access) |
| **fortify-http CPU** | ~15-18% (handles CAPTCHA HTML) |
| **fortify-gate CPU** | <5% (minimal load) |

---

## Security Considerations

### ✅ Security Maintained or Improved:

1. **CAPTCHA Verification:** Still handled by Gate (trusted component)
2. **Session Token Creation:** Still handled by Gate (cryptographic security)
3. **No New Attack Surface:** fortify-http only serves static HTML template
4. **Rate Limiting:** Still active, now faster to respond

### ⚠️ Potential Concerns:

#### 1. CAPTCHA ID Predictability
- **Concern:** Attacker could guess CAPTCHA IDs
- **Mitigation:** UUIDv4 (2^122 space, cryptographically random)
- **Risk:** NEGLIGIBLE

#### 2. CAPTCHA Solving Without Loading Image
- **Concern:** Attacker submits answer without viewing image
- **Mitigation:** Gate validates CAPTCHA ID exists and answer is correct
- **Risk:** LOW (Gate still controls verification)

#### 3. CAPTCHA HTML Injection
- **Concern:** Attacker injects malicious HTML via reason parameter
- **Mitigation:** Sanitize reason parameter, use safe HTML escaping
- **Risk:** LOW (easy to mitigate)

#### 4. Bypass Gate Proxy Entirely
- **Concern:** Attacker tries to access Gate directly
- **Mitigation:** Gate already listens on 127.0.0.1 only (localhost)
- **Risk:** NONE (no change from current)

---

## Rollback Plan

If implementation causes issues:

1. **Revert Phase 2-4 changes:**
   - Change `serve_captcha_html()` calls back to `proxy_to_gate()`
   - Restore redirect to `/Fortify/Portcullis`

2. **Keep Phase 1 (template):**
   - No harm in having template available
   - Can be used for future features

3. **Rebuild and deploy:**
   ```bash
   cargo build --release
   ./target/release/fortify
   ```

4. **Verify rollback:**
   ```bash
   curl -I http://127.0.0.1:8080/
   # Should see: Location: /Fortify/Portcullis (old behavior)
   ```

---

## Dependencies

### New Crate Dependencies:

**`crates/fortify-http/Cargo.toml`:**
```toml
uuid = { version = "1.6", features = ["v4", "fast-rng"] }
```

### Existing Dependencies (no change):
- hyper (HTTP server)
- tokio (async runtime)
- All current fortify-* crates

---

## Implementation Checklist

### Phase 1: Preparation
- [ ] Add `uuid` dependency to `fortify-http/Cargo.toml`
- [ ] Add `CAPTCHA_HTML_TEMPLATE` constant to `lib.rs`
- [ ] Add `serve_captcha_html()` function to `lib.rs`
- [ ] Build and verify compilation: `cargo build`

### Phase 2: Update Rate Limit Flow
- [ ] Locate rate limit redirect code (~line 620)
- [ ] Replace redirect with `serve_captcha_html()`
- [ ] Test rate limiting: `for i in {1..15}; do curl http://127.0.0.1:8080/; done`

### Phase 3: Update Unknown User Flow
- [ ] Locate Unknown user proxy code (~line 690)
- [ ] Replace proxy with `serve_captcha_html()`
- [ ] Test new user flow: `curl http://127.0.0.1:8080/`

### Phase 4: Update Blacklist Flow
- [ ] Locate blacklist redirect code (~line 940)
- [ ] Replace redirect with `serve_captcha_html()`
- [ ] Test blacklist flow (simulate blacklisted session)

### Phase 5: Gate Connection Limits (Optional)
- [ ] Add Semaphore to `fortify-gate/lib.rs`
- [ ] Implement connection limiting logic
- [ ] Test with connection flood

### Phase 6: Testing
- [ ] Run all 5 test scenarios
- [ ] Monitor logs during testing
- [ ] Verify performance metrics
- [ ] Check for any errors or warnings

### Phase 7: Documentation
- [ ] Update RATE_LIMITING.md with new flow
- [ ] Update SECURITY_AUDIT.md
- [ ] Create this dev progress document ✓

---

## Success Criteria

Implementation is successful when:

1. ✅ **User Experience:** CAPTCHA loads in <1 second during attacks
2. ✅ **No Hanging:** Zero users report 30+ second hangs
3. ✅ **Gate Load:** <1,500 Gate requests during 3,500 req/sec attack
4. ✅ **CPU Usage:** fortify-http stays below 50% CPU
5. ✅ **Security:** All CAPTCHA verifications still handled by Gate
6. ✅ **No Errors:** No new errors in logs during attack simulations

---

## Timeline

- **Planning/Documentation:** 1 hour ✓ (this document)
- **Implementation (Phases 1-4):** 30-45 minutes
- **Testing:** 30 minutes
- **Monitoring/Validation:** 1 hour
- **Total:** ~3 hours

---

## Notes

- This is a **critical security improvement** - fixes Gate bottleneck that prevents real users from accessing site during attacks
- Implementation is **low-risk** - mostly changes response generation, doesn't affect core verification logic
- **Backward compatible** - Gate still handles all verification, just less page serving
- **Tor-friendly** - No reliance on IP addresses or clearnet assumptions
- **Performance win** - 600x faster CAPTCHA response time

---

**Ready for Implementation:** YES  
**Approval Required:** YES  
**Risk Level:** LOW  
**Impact:** HIGH (significantly improves user experience during attacks)
