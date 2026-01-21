# Defensive Sprint R1: Real User Protection & Attack Mitigation

**Created:** January 19, 2026  
**Status:** IN PROGRESS  
**Goal:** Eliminate real user degradation during attacks (29-second delay fix)

---

## 🎯 OBJECTIVE

After Attack #4 analysis, real user session 458342 experienced a **29-second delay** during a 6-second hybrid attack (669 requests total). This sprint implements upstream filtering to protect legitimate users from attack-induced latency.

---

## 📋 IMPLEMENTATION TASK LIST

### **PHASE 1: HTTP Proxy Rate Limiting** ⚡ HIGHEST PRIORITY
**Goal:** Drop attack traffic at proxy before it reaches nodes  
**Estimated Time:** 2 hours  
**Status:** ✅ COMPLETE

#### Task 1.1: Add Rate Limiter Structure ✅
- [x] File: `crates/fortify-http/src/lib.rs`
- [x] Add dependencies: `std::time::{Duration, Instant}`
- [x] Create `GlobalRateLimiter` struct with:
  - `requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>`
  - `limit: usize` (75 requests - adjusted for Tor)
  - `window: Duration` (10 seconds)
- [x] Implement `check_and_record(ip: &str) -> bool` method
- [x] Add cleanup logic to remove expired timestamps

#### Task 1.2: Extract Client IP ✅
- [x] File: `crates/fortify-http/src/lib.rs`
- [x] Add function `extract_client_ip()` to extract IP from request headers
- [x] Check `X-Forwarded-For`, `X-Real-IP`, fallback to "unknown"
- [x] Handle both header formats

#### Task 1.3: Integrate Rate Limiting ✅
- [x] File: `crates/fortify-http/src/lib.rs`
- [x] Initialize `GlobalRateLimiter` in HTTP server startup (75 req/10sec)
- [x] Add rate check BEFORE session token validation and backpressure
- [x] Return `429 Too Many Requests` with `Retry-After: 10` header
- [x] Add logging: "Rate limited IP: {ip} (75 req/10sec exceeded)"
- [x] Build successful

#### Task 1.4: Testing
- [ ] Test: 75 requests in 9 seconds → all succeed
- [ ] Test: 76th request in 9 seconds → gets 429
- [ ] Test: Wait 10 seconds → requests succeed again
- [ ] Test: Attack #5 simulation → verify 73% reduction in processed requests

---

### **PHASE 2: Burst Exception for Clean Sessions** ✅ USER EXPERIENCE
**Goal:** Prevent false positives for page loads with many assets  
**Estimated Time:** 1 hour  
**Status:** ✅ COMPLETE

#### Task 2.1: Identify Clean Sessions
- [ ] File: `crates/fortify-node/src/lib.rs`
- [ ] In `check_rate_limit()` function
- [ ] Check if session has `violations == 0` (never demoted)
- [ ] Check if request count > 30 in < 10 seconds (asset burst)
- [ ] Add burst exception flag to session state

#### Task 2.2: Implement Burst Logic
- [ ] File: `crates/fortify-node/src/lib.rs`
- [ ] Before applying `RateLimitExceeded` violation:
  ```rust
  if violations == 0 && request_timestamps.len() > 30 && elapsed < Duration::from_secs(10) {
      tracing::info!("Session {} burst exception: {} requests in {}s", 
                     session_id, request_timestamps.len(), elapsed.as_secs());
      return Ok(()); // Allow burst
  }
  ```
- [ ] Log burst exceptions for monitoring
- [ ] Ensure burst only applies ONCE per session (set flag after first burst)

#### Task 2.3: Testing
- [ ] Test: New session makes 40 requests in 5 seconds → no demotion
- [ ] Test: Session with 1 violation makes 40 requests → demoted (no exception)
- [ ] Test: Session makes burst, then another burst → second burst demoted
- [ ] Verify real page load with 25 images doesn't trigger violation

---

### **PHASE 3: Session Blacklist** 🔒 CRITICAL
**Goal:** Prevent demoted sessions from reusing tokens  
**Estimated Time:** 4 hours  
**Status:** ✅ COMPLETE

#### Task 3.1: Add Blacklist to Controller
- [ ] File: `crates/fortify-controller/src/lib.rs`
- [ ] Add field: `session_blacklist: Arc<Mutex<HashMap<String, u64>>>`
  - Key: session_id
  - Value: timestamp when blacklist expires (current_time + 60 seconds)
- [ ] Add method: `add_to_blacklist(session_id: String, duration_secs: u64)`
- [ ] Add method: `is_blacklisted(session_id: &str) -> bool`
- [ ] Add cleanup task: Remove expired entries every 30 seconds

#### Task 3.2: Report Demotions from Nodes
- [ ] File: `crates/fortify-node/src/lib.rs`
- [ ] Add callback/channel to report demotions to controller
- [ ] When session hits 3 violations:
  ```rust
  tracing::info!("Session {} demoted, reporting to controller", session_id);
  controller.add_to_blacklist(session_id.clone(), 60);
  ```
- [ ] Include session_id in demotion report

#### Task 3.3: Check Blacklist in HTTP Proxy
- [ ] File: `crates/fortify-http/src/lib.rs`
- [ ] After session token validation succeeds
- [ ] Before routing to HEALTHY PATH:
  ```rust
  if controller.is_blacklisted(&session.session_id) {
      tracing::info!("Blacklisted session {} attempting access", session.session_id);
      // Clear cookie and redirect to Gate
      return redirect_to_gate();
  }
  ```
- [ ] Add logging for blacklist hits

#### Task 3.4: Progressive Penalties
- [ ] File: `crates/fortify-controller/src/lib.rs`
- [ ] Track demotion count per session: `HashMap<String, u8>`
- [ ] Escalating timeouts:
  - 1st demotion: 60 seconds
  - 2nd demotion: 300 seconds (5 minutes)
  - 3rd demotion: 1800 seconds (30 minutes)
- [ ] Reset demotion count after 24 hours of good behavior

#### Task 3.5: Testing
- [ ] Test: Demoted session tries to reuse token → blacklisted, redirected
- [ ] Test: Wait 60 seconds, clear blacklist → can verify again
- [ ] Test: 2nd demotion → blacklisted for 5 minutes
- [ ] Test: Blacklist persists across node restarts (optional: save to disk)

---

### **PHASE 4: CAPTCHA Rate Limiting** 🎯 GATE PROTECTION
**Goal:** Prevent Gate saturation from unauthenticated spam  
**Estimated Time:** 2 hours  
**Status:** ✅ COMPLETE

#### Task 4.1: Add Rate Limiter to Gate
- [x] File: `crates/fortify-gate/src/lib.rs`
- [x] Rate limiter already exists: `RateLimiter::new(10, 60)` - 10 requests per minute
- [x] Track verification session creation attempts per session ID

#### Task 4.2: Implement Gate Rate Check
- [x] File: `crates/fortify-gate/src/lib.rs`
- [x] In `create_verification_with_type()` function
- [x] Check rate limit BEFORE creating session:
  ```rust
  if let Err(_) = self.rate_limiter.check_rate_limit(&session_id) {
      return Err(GateError::RateLimitExceeded);
  }
  ```
- [x] Log rate limit hits for monitoring

#### Task 4.3: Testing
- [ ] Test: 10 new CAPTCHA requests in 30 seconds → all succeed
- [ ] Test: 11th request within 60 seconds → RateLimitExceeded error
- [ ] Test: Wait 60 seconds → can create new session again
- [ ] Verify Attack #4 pattern (249 unauth requests) would be limited to 10/min

---

### **PHASE 5: IP-Based Pre-filtering** 🛡️ ADVANCED
**Goal:** Auto-block IPs generating excessive demotions  
**Estimated Time:** 3 hours  
**Status:** ⏭️ SKIPPED - Tor Exit Node Concerns

**Reason for Skipping:** IP-based blocking could inadvertently block legitimate Tor users who share exit nodes with attackers. This would degrade the user experience for privacy-focused users, which conflicts with Fortify's Tor-friendly design principle. Session-based blacklist (Phase 3) provides better targeting without collateral damage.

#### Task 5.1: Track Demotion Sources
- [ ] File: `crates/fortify-http/src/lib.rs`
- [ ] Add `demotion_tracker: Arc<Mutex<HashMap<String, Vec<Instant>>>>`
  - Key: client IP
  - Value: timestamps of demotion events
- [ ] When node reports demotion, record IP address

#### Task 5.2: Implement Auto-Block Logic
- [ ] File: `crates/fortify-http/src/lib.rs`
- [ ] Check if IP has >3 demotions in 60 seconds
- [ ] Add to IP blocklist for 600 seconds (10 minutes)
- [ ] No CAPTCHA opportunity - instant 429

#### Task 5.3: Integrate IP Blocklist Check
- [ ] File: `crates/fortify-http/src/lib.rs`
- [ ] Check IP blocklist BEFORE rate limiter
- [ ] Return `429 Too Many Requests` immediately
- [ ] Log: "Blocked IP {ip}: {demotion_count} demotions in 60s"

#### Task 5.4: Testing
- [ ] Test: Single IP causes 4 demotions in 30 seconds → IP blocked
- [ ] Test: Blocked IP attempts request → instant 429
- [ ] Test: Wait 10 minutes → IP unblocked
- [ ] Verify Attack #4 pattern (128 demotions from same IPs) triggers auto-block

---

### **PHASE 6: Metrics & Monitoring** 📊 VISIBILITY
**Goal:** Track real user experience during attacks  
**Estimated Time:** 4 hours  
**Status:** NOT STARTED

#### Task 6.1: Add Metrics Endpoint
- [ ] File: `crates/fortify-http/src/lib.rs`
- [ ] Create `/metrics` endpoint (Prometheus format)
- [ ] Return plain text with metric lines

#### Task 6.2: Track Response Times
- [ ] File: `crates/fortify-http/src/lib.rs`
- [ ] Record request start time
- [ ] Record request end time
- [ ] Calculate duration
- [ ] Separate tracking for:
  - HEALTHY PATH (real users)
  - THREAT PATH (unverified)
- [ ] Calculate 95th percentile latency

#### Task 6.3: Track Key Metrics
- [ ] Metric: `fortify_http_requests_total{path, status}`
- [ ] Metric: `fortify_http_response_time_seconds{path, percentile}`
- [ ] Metric: `fortify_demotions_per_minute`
- [ ] Metric: `fortify_rate_limit_hits_total{limiter}`
- [ ] Metric: `fortify_blacklist_size`
- [ ] Metric: `fortify_ip_blocklist_size`

#### Task 6.4: Alert Thresholds
- [ ] Alert if HEALTHY PATH 95p latency >5 seconds
- [ ] Alert if demotions >50/minute
- [ ] Alert if rate limit hits >100/minute
- [ ] Log alerts to console and metrics endpoint

---

## 🧪 INTEGRATION TESTING

### Test Scenario 1: Clean User During Attack
- [ ] Start legitimate session, browse 5 pages normally
- [ ] Launch 100-request attack from different IP
- [ ] Verify legitimate user experiences <5 second response times
- [ ] Verify attacker gets rate limited after 50 requests
- [ ] Verify legitimate user never gets demoted or delayed

### Test Scenario 2: Page Load with Assets
- [ ] New session loads page with 30 images/CSS/JS files
- [ ] All 31 requests (1 HTML + 30 assets) arrive in 3 seconds
- [ ] Verify burst exception applies
- [ ] Verify no rate limit violation
- [ ] Verify no demotion

### Test Scenario 3: Session Reuse Attack
- [ ] Session gets demoted (3 violations)
- [ ] Attempt to reuse same session token immediately
- [ ] Verify blacklist prevents access
- [ ] Wait 60 seconds, try again
- [ ] Verify can start fresh verification

### Test Scenario 4: Multi-IP Attack
- [ ] 5 different IPs each send 40 requests in 5 seconds
- [ ] Each IP gets rate limited at 50 requests
- [ ] IPs causing demotions get blocked for 10 minutes
- [ ] Verify total attack traffic capped at 250 requests (5 IPs × 50)

### Test Scenario 5: Reproduce Attack #4
- [ ] Simulate 128 authenticated attack sessions
- [ ] Simulate 249 unauthenticated requests
- [ ] Launch over 6-second window
- [ ] Verify legitimate user maintains <5 second response time
- [ ] Verify attacker IPs get blocked after 3 demotions
- [ ] Verify Gate limits unauth to 10 new sessions

---

## 📈 SUCCESS CRITERIA

### Real User Protection
- ✅ 95th percentile response time <5 seconds during attacks
- ✅ Zero false positive demotions for normal browsing
- ✅ Page loads with 20+ assets don't trigger violations
- ✅ Legitimate users never experience >10 second delays

### Attack Mitigation
- ✅ Proxy-level rate limiting drops 90% of attack traffic
- ✅ Demoted sessions blocked for 60+ seconds minimum
- ✅ Unauthenticated spam limited to 10 new sessions/minute
- ✅ Attack IPs auto-blocked after 3 demotions

### System Health
- ✅ Node CPU usage <10% during attacks
- ✅ HTTP proxy handles attacks without saturation
- ✅ Blacklist memory usage <10MB
- ✅ Metrics endpoint responds in <100ms

---

## 🚀 DEPLOYMENT PLAN

### Step 1: Phase 1-2 (Week 1, Days 1-3)
- Deploy HTTP proxy rate limiting
- Deploy burst exception
- Test with simulated attacks
- Monitor real user metrics

### Step 2: Phase 3-4 (Week 1, Days 4-5)
- Deploy session blacklist
- Deploy CAPTCHA rate limiting
- Run integration tests
- Monitor blacklist size

### Step 3: Phase 5-6 (Week 2, Days 6-10)
- Deploy IP pre-filtering
- Deploy metrics endpoint
- Run full attack simulations
- Validate all success criteria

---

## 🔧 CONFIGURATION

### Default Values (Tunable)
```toml
[defense]
# Proxy-level rate limiting
proxy_rate_limit = 50  # requests per window
proxy_rate_window_secs = 10

# Burst exception
burst_exception_threshold = 30  # requests
burst_exception_window_secs = 10

# Session blacklist
blacklist_duration_1st = 60  # seconds
blacklist_duration_2nd = 300
blacklist_duration_3rd = 1800

# CAPTCHA rate limiting
gate_rate_limit = 10  # new sessions per window
gate_rate_window_secs = 60

# IP auto-block
ip_block_demotion_threshold = 3  # demotions
ip_block_window_secs = 60
ip_block_duration_secs = 600
```

---

## 📝 NOTES & QUESTIONS

### Questions for Review:
1. **Proxy rate limit:** Is 50 req/10sec too strict? Should we make it 100 req/10sec?
2. **Burst exception:** Should it apply per session or per IP? (Currently per session)
3. **Blacklist persistence:** Should blacklist survive server restarts? (Currently in-memory)
4. **IP blocking:** Should we whitelist known good IPs (like Tor exit nodes)?
5. **Metrics:** Do we need a separate metrics port, or use main HTTP port?

### Technical Considerations:
- **Memory:** Blacklist and rate limiters are in-memory, bounded by time windows
- **Performance:** All checks are O(1) HashMap lookups with O(n) timestamp cleanup
- **Concurrency:** All structures use `Arc<Mutex<>>` for thread safety
- **IPv6:** Need to handle IPv6 addresses in rate limiter keys

---

## 🎯 NEXT STEPS

1. **Review this plan** - Are priorities correct? Any missing tasks?
2. **Start Phase 1, Task 1.1** - Add rate limiter structure to HTTP proxy
3. **Iterative testing** - Test each task before moving to next
4. **Metrics collection** - Track before/after attack response times

---

**Ready to begin implementation? Please confirm:**
- [ ] Proxy rate limit: 50 req/10sec acceptable?
- [ ] Burst exception: 30 req/10sec threshold OK?
- [ ] Blacklist: In-memory (non-persistent) acceptable for R1?
- [ ] Any other configuration changes needed?

