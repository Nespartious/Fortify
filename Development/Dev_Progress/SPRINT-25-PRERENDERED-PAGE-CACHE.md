# Sprint 25: Pre-Rendered Page Cache Implementation

**Status:** 🔴 CRITICAL BUG - Ready for Implementation  
**Priority:** HIGHEST  
**Started:** 2026-01-26  
**Last Updated:** 2026-01-26

---

## Problem Statement

### Current Broken Behavior
During attacks, users see "⚠ Service Temporarily Unavailable" 503 error page instead of CAPTCHA challenges.

### Root Cause Analysis (VERIFIED)

**Investigation confirmed the following issues:**

#### Issue 1: Pre-rendered API Doesn't Use Pool for Full Pages
- **File:** [fortify-gate/src/server.rs#L350-L380](fortify/crates/fortify-gate/src/server.rs#L350-L380)
- **Problem:** `serve_prerendered_page_api()` calls `gate.create_verification_with_type()` which creates a NEW verification session for every request
- **Evidence:** Line 360 shows direct call to `create_verification_with_type()` - no pool lookup

#### Issue 2: Verification Session Limit is the Bottleneck
- **File:** [fortify-gate/src/lib.rs#L613](fortify/crates/fortify-gate/src/lib.rs#L613)
- **Problem:** `max_concurrent_verifications = 10` (default!) limits pending sessions
- **Code:** `if states.len() >= self.max_concurrent { return Err(GateError::QueueFull) }`
- **Impact:** Under 100 req/sec attack, limit hit in 0.1 seconds → `QueueFull` → 500 → 503

#### Issue 3: Raw CAPTCHA Pool Exists But HTML Pool Doesn't
- **What EXISTS:** `captcha_pool: Arc<Mutex<Vec<CaptchaChallenge>>>` - raw images
- **What EXISTS:** `take_captcha()` method pulls from pool (line 505)
- **What's MISSING:** No pool of **complete HTML pages** ready to serve
- **What's MISSING:** No state tracking (Available/InUse/Solved)

#### Issue 4: Session ID Collision Risk
- Session IDs are embedded in HTML at generation time
- If same page served to 2 users → both submit same session_id → collision
- **Original Design (Sprint 13):** Lazy registration solves this

### Full Request Flow Under Attack

```
1. User request → HTTP Proxy
2. HTTP Proxy: "Rate limited, serve cached CAPTCHA"
3. HTTP Proxy calls: GET /gate/api/prerendered-page
4. Gate API: serve_prerendered_page_api()
5. Gate API: create_verification_with_type() ← GENERATES NEW SESSION
6. Gate API: verification_states.len() >= max_concurrent (10)
7. Gate API: return Err(GateError::QueueFull)
8. HTTP Proxy: 500 error, fallback to proxy
9. Gate: serve_landing_page() also hits QueueFull
10. Gate: Returns 503 "Service Temporarily Unavailable"
11. User: Never sees CAPTCHA ❌
```

---

## Solution Design

### Architecture: Large Reusable Pool with State Tracking

**Pool Size:** 500-1000 pre-generated HTML pages (configurable)

```rust
// fortify-gate/src/prerendered_pool.rs (NEW FILE)

pub struct PrerenderedPagePool {
    pages: Arc<Mutex<Vec<PooledPage>>>,
    answers: Arc<Mutex<HashMap<String, String>>>,  // captcha_id → answer
    config: PoolConfig,
    metrics: PoolMetrics,
}

struct PooledPage {
    captcha_id: String,       // Unique identifier
    html: String,             // Complete HTML page
    state: PageState,
    generated_at: u64,
}

enum PageState {
    Available,                            // Ready to serve
    InUse {
        session_id: String,              // User's session
        served_at: u64,                  // Timestamp
    },
    Solved,                              // Needs regeneration
}

struct PoolConfig {
    target_size: usize,                  // 500-1000
    timeout_seconds: u64,                // 120 seconds
    max_age_seconds: u64,                // 600 seconds (10 min)
}
```

### Serving Logic

```rust
fn serve_prerendered_page_api(gate: Arc<Gate>) -> Response<BoxBody> {
    // 1. Try to serve from pool (INSTANT - O(1))
    if let Some(page) = gate.prerendered_pool.take_available() {
        // Assign unique session_id NOW (not at generation time)
        let session_id = uuid::Uuid::new_v4().to_string();
        
        // Mark page as in-use (prevents serving to another user)
        gate.prerendered_pool.mark_in_use(&page.captcha_id, &session_id);
        
        // Register answer for lazy verification
        gate.register_pending_captcha(&page.captcha_id, &session_id);
        
        // Inject session_id into HTML (template replacement)
        let html = page.html.replace("{{SESSION_ID}}", &session_id);
        
        return build_cached_response(html, session_id);
    }
    
    // 2. Pool exhausted - generate on-demand (should be rare)
    tracing::warn!("Pre-rendered pool exhausted!");
    generate_page_immediately(gate)
}
```

### Lazy Session Registration (from Sprint 13)

When user submits answer, Gate creates session on-the-fly:

```rust
pub fn verify_captcha(&self, session_id: &str, solution: &str) -> Result<()> {
    // Check existing sessions first
    if let Some(state) = self.verification_states.get(session_id) {
        return self.verify_existing_session(state, solution);
    }
    
    // LAZY REGISTRATION: Check if this is from the pool
    if let Some(expected_answer) = self.pending_captchas.get(session_id) {
        let is_correct = expected_answer.eq_ignore_ascii_case(solution);
        
        if is_correct {
            // Create session, mark page as Solved
            self.prerendered_pool.mark_solved(session_id);
            return Ok(());
        } else {
            return Err(GateError::InvalidCaptcha);
        }
    }
    
    Err(GateError::ChallengeNotFound)
}
```

### Dynamic Resource Management (Traffic-Aware)

```rust
struct PoolMetrics {
    requests_per_minute: AtomicU64,
    pool_available: AtomicUsize,
    pool_in_use: AtomicUsize,
    generation_rate: AtomicU64,
}

fn calculate_generation_rate(&self) -> u64 {
    let rpm = self.metrics.requests_per_minute.load(Ordering::Relaxed);
    let utilization = self.get_utilization_percent();
    
    // Low traffic → generate slowly (save resources)
    // High traffic → generate fast (maintain pool)
    match (rpm, utilization) {
        (0..=10, _)        => 2,    // Idle: 2 pages/sec
        (11..=50, 0..=50)  => 5,    // Light load: 5/sec
        (11..=50, 51..=80) => 10,   // Medium load: 10/sec
        (51..=100, _)      => 25,   // Heavy load: 25/sec
        (101.., _)         => 50,   // Attack: 50/sec MAX
    }
}
```

### Background Maintenance Task

```rust
async fn pool_maintenance_loop(pool: Arc<PrerenderedPagePool>) {
    loop {
        // 1. Reclaim timed-out InUse pages → Available
        pool.reclaim_expired_pages();
        
        // 2. Regenerate Solved pages
        pool.regenerate_solved_pages().await;
        
        // 3. Fill pool to target size
        let needed = pool.config.target_size - pool.available_count();
        let rate = pool.calculate_generation_rate();
        let batch_size = (rate * 5).min(needed);  // 5-second batches
        
        for _ in 0..batch_size {
            if let Some(page) = generate_pooled_page().await {
                pool.add_page(page);
            }
        }
        
        // 4. Update metrics
        pool.update_metrics();
        
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
```

---

## Implementation Phases

### Phase 1: PrerenderedPagePool Struct
- [ ] Create `prerendered_pool.rs` in fortify-gate
- [ ] Implement `PooledPage`, `PageState`, `PoolConfig`
- [ ] Implement `take_available()`, `mark_in_use()`, `mark_solved()`
- [ ] Add unit tests
- **Checkpoint:** `cargo test --package fortify-gate` passes

### Phase 2: Integrate Pool into Gate
- [ ] Add `prerendered_pool: Arc<PrerenderedPagePool>` to Gate struct
- [ ] Initialize pool with configurable target size
- [ ] Add `pending_captchas: Arc<Mutex<HashMap<String, String>>>`
- **Checkpoint:** Gate starts with empty pool, no errors

### Phase 3: Update API Endpoint
- [ ] Modify `serve_prerendered_page_api()` to serve from pool
- [ ] Implement session_id injection into HTML
- [ ] Implement `register_pending_captcha()`
- **Checkpoint:** API returns cached pages (check logs)

### Phase 4: Lazy Verification
- [ ] Update `verify_captcha()` to check `pending_captchas`
- [ ] Create session on successful verification
- [ ] Mark page as Solved
- **Checkpoint:** User can solve pooled CAPTCHA

### Phase 5: Background Maintenance
- [ ] Spawn maintenance task on Gate startup
- [ ] Implement `reclaim_expired_pages()`
- [ ] Implement `regenerate_solved_pages()`
- [ ] Implement traffic-aware generation rate
- **Checkpoint:** Pool maintains target size under load

### Phase 6: Testing & Validation
- [ ] Stress test with 100 req/sec
- [ ] Verify zero 503 errors
- [ ] Verify pool metrics in logs
- [ ] Test timeout reclamation
- **Checkpoint:** All tests pass

---

## Configuration

```toml
[gate.prerendered_pool]
target_size = 500                    # Number of pre-generated pages
timeout_seconds = 120                # InUse timeout (return to Available)
max_age_seconds = 600                # Maximum page age before regeneration
initial_fill = true                  # Fill pool on startup

[gate.prerendered_pool.generation]
idle_rate = 2                        # Pages/sec when traffic < 10 req/min
low_rate = 5                         # Pages/sec when traffic 10-50 req/min
medium_rate = 10                     # Pages/sec when traffic 50-100 req/min
high_rate = 25                       # Pages/sec when traffic 100-500 req/min
attack_rate = 50                     # Pages/sec when traffic > 500 req/min
```

---

## Success Criteria

- [ ] Pool initialized with 500+ pages on startup
- [ ] API endpoint serves from pool (logs show "Serving from pool")
- [ ] Session_id assigned at serve time (not generation)
- [ ] InUse pages return to Available after timeout
- [ ] Solved pages regenerated in background
- [ ] Generation rate adjusts based on traffic
- [ ] Zero 503 errors during stress test (100 req/sec for 60 seconds)
- [ ] Pool metrics visible in logs/admin

---

## Risk Analysis

| Risk | Impact | Mitigation |
|------|--------|------------|
| Memory usage (500 pages × 50KB = 25MB) | Low | Acceptable for server |
| Timeout too short (users can't solve) | High | Default 120s, configurable |
| Timeout too long (pool depletes) | Medium | Monitor utilization, adjust |
| Generation can't keep up with attacks | High | Pre-fill to 500+, rate up to 50/sec |
| Session_id injection fails | Critical | Template uses `{{SESSION_ID}}` marker |

---

## Notes

### What Exists But Isn't Connected
- Orchestrator has `CaptchaPoolManager` with `take_prerendered_page()` (line 1448)
- Core has `PrerenderedCaptchaPage` struct (templates.rs:361)
- Gate has `captcha_pool` for raw images (lib.rs:423)
- Sprint 13 documented this design but was marked "NOT STARTED"

### Why Previous Attempts Failed
1. Raw CAPTCHA pool was built but not HTML page pool
2. API endpoint calls `create_verification_with_type()` directly
3. `max_concurrent_verifications = 10` limit too low
4. Session registration requires session BEFORE page serve
5. No state tracking (Available/InUse/Solved)

### This Implementation Fixes All Issues
1. ✅ Pool stores complete HTML pages
2. ✅ API serves from pool, no verification creation
3. ✅ Bypasses verification session limit
4. ✅ Lazy registration - session created on verify
5. ✅ State tracking prevents serving same page twice

---

## References

- [Sprint 13 - Combined CAPTCHA Landing Page](archive/13-COMBINED-CAPTCHA-LANDING-SPRINT.md)
- [Orchestrator CaptchaPoolManager](../fortify/crates/fortify-orchestrator/src/lib.rs#L1290)
- [Gate API Endpoint](../fortify/crates/fortify-gate/src/server.rs#L350)
- [Verification Session Limit](../fortify/crates/fortify-gate/src/lib.rs#L613)
