# Sprint: Pre-Production Hardening

**Sprint ID:** BETA-003  
**Priority:** 🟡 MEDIUM (Pre-Production)  
**Estimated Effort:** 2-3 days  
**Status:** 📋 PLANNING (Under Discussion)  
**Created:** January 23, 2026  
**Last Updated:** January 22, 2026

---

## ⚠️ PLANNING NOTES

This sprint is under active discussion to ensure implementation aligns with service goals.

### Service Goals (Priority Order)

1. **99% uptime for existing verified/trusted sessions** - Even under extreme attack
2. **75% access rate for new legitimate users** - Can reach CAPTCHA and solve
3. **Attack traffic isolated** - Routed to their own resource pool, shed first

### Key Architectural Decisions (Under Discussion)

**Load Shedding Priority:**
```
┌─────────────────────────────────────────────────────────────────────┐
│  PRIORITY 1 (NEVER SHED): Trusted Sessions                         │
│  PRIORITY 2 (SHED LAST):  Verified Sessions                        │
│  PRIORITY 3 (SHED FIRST): Unknown/Suspicious (Threat Path)         │
└─────────────────────────────────────────────────────────────────────┘
```

**Question Under Review:** Should 503 only apply to threat-tier sessions while healthy sessions remain unaffected?

**Current Thinking:**
- Healthy nodes serve ONLY verified/trusted → protect at all costs
- Threat-tier sessions get 503 first when system is stressed
- New users route to Gate which serves CACHED/STATIC CAPTCHAs (nearly zero CPU)
- Only `/verify` endpoint does real processing (rate-limit this separately)

### Existing Implementation to Leverage

**CAPTCHA Pre-generation (Already Implemented):**
- `CaptchaPoolManager` in `fortify-orchestrator/src/lib.rs`
- Default: 500 pre-generated, min 100, max 1000
- CPU-aware: pauses at 70% CPU usage
- Persisted to disk: `captcha_pool.json`
- Rotation: 25% every 10 days

**Proposed Optimization (from Alpha_Review):**
> "Serve CAPTCHA HTML directly from fortify-http (eliminate Gate bottleneck)"
> "Only proxy verification to Gate (97% load reduction)"

This means fortify-http can serve the CAPTCHA page as a static/cached asset, dramatically reducing Gate load and making it nearly impossible for attackers to exhaust resources.

---

## Objective

Implement additional security hardening measures identified in the external security review. These are not Beta Blockers but should be completed before production deployment.

---

## Background

An external security review of our Panic Audit Strategy identified gaps in:
1. Concurrency control (no global semaphore gating)
2. Graceful degradation (no 503 on overload)
3. Timing fingerprint resistance (fixed timeouts)
4. Tor hidden service configuration (missing DoS defense options)

See [SECURITY-REVIEW-COMPARISON.md](SECURITY-REVIEW-COMPARISON.md) for full gap analysis.

---

## Success Criteria

- [ ] Global concurrency semaphore limits total connections
- [ ] System returns 503 when at capacity (not timeout)
- [ ] All timeouts have ±10-20% jitter
- [ ] Tor hidden services configured with IntroDoSDefense and MaxStreams

---

## Implementation Tasks

### Task 1: Global Concurrency Semaphore
**Status:** ⬜ Not Started  
**Estimated Time:** 4 hours  
**Priority:** 🔴 HIGH

**Problem:** Current implementation uses soft counters, not actual semaphore gating. Under extreme load, more connections than `max_connections` could be active simultaneously due to race conditions.

**Files to Modify:**
- `crates/fortify-http/src/lib.rs`
- `crates/fortify-http/src/proxy.rs`

**Current Code:**
```rust
pub struct BackendNode {
    address: String,
    healthy_mode: bool,
    active_connections: RwLock<usize>,
    max_connections: usize,
}
```

**Required Change:**
```rust
use tokio::sync::Semaphore;

// Global limit across all backend nodes
static GLOBAL_CONNECTION_LIMIT: tokio::sync::Semaphore = 
    tokio::sync::Semaphore::const_new(1000);

pub struct BackendNode {
    address: String,
    healthy_mode: bool,
    connection_semaphore: Arc<Semaphore>,
    max_connections: usize,
}

impl BackendNode {
    pub async fn acquire(&self) -> Option<SemaphorePermit> {
        // First check global limit
        let _global = GLOBAL_CONNECTION_LIMIT.try_acquire().ok()?;
        // Then per-node limit
        self.connection_semaphore.try_acquire().ok()
    }
}
```

**Sub-tasks:**
- [ ] Add tokio Semaphore to BackendNode
- [ ] Create global connection semaphore
- [ ] Update try_acquire() to use actual semaphore
- [ ] Update release() to drop permit
- [ ] Add tests for concurrent access

---

### Task 2: Graceful 503 on Overload
**Status:** ⬜ Not Started  
**Estimated Time:** 2 hours  
**Priority:** 🔴 HIGH

**Problem:** When all nodes are at capacity, requests may queue indefinitely instead of failing fast with 503.

**Files to Modify:**
- `crates/fortify-http/src/routing.rs`
- `crates/fortify-http/src/proxy.rs`

**Current Behavior:** Select least-loaded node, even if all are overloaded.

**Required Change:**
```rust
pub fn route_request(&self, trust_tier: TrustTier) -> Result<&BackendNode, HttpError> {
    let nodes = self.get_nodes_for_tier(trust_tier);
    
    for node in nodes.iter().sorted_by_key(|n| n.active_connections()) {
        if node.has_capacity() {
            return Ok(node);
        }
    }
    
    // All nodes at capacity - return 503
    Err(HttpError::ServiceUnavailable("All nodes at capacity"))
}
```

**Response:**
```http
HTTP/1.1 503 Service Unavailable
Retry-After: 5
Content-Type: text/html

<html>
<head><title>Service Busy</title></head>
<body>
<h1>Service Temporarily Unavailable</h1>
<p>The service is experiencing high demand. Please try again in a few seconds.</p>
</body>
</html>
```

**Sub-tasks:**
- [ ] Add `ServiceUnavailable` error variant
- [ ] Update routing to check capacity before selecting
- [ ] Create `503.html` template
- [ ] Add Retry-After header (jittered value)
- [ ] Add metrics for 503 responses

---

### Task 3: Timeout Jitter
**Status:** ⬜ Not Started  
**Estimated Time:** 2 hours  
**Priority:** 🟡 MEDIUM

**Problem:** Fixed timeout values can be fingerprinted by attackers to identify Fortify-protected services.

**Files to Modify:**
- `crates/fortify-http/src/lib.rs`
- `crates/fortify-http/src/proxy.rs`
- `crates/fortify-orchestrator/src/tor.rs`
- `crates/fortify-gate/src/server.rs`

**Current Code:**
```rust
const BACKEND_REQUEST_TIMEOUT_SECS: u64 = 60;
```

**Required Change:**
```rust
use rand::Rng;

/// Add ±15% jitter to a timeout value
fn jittered_timeout(base_secs: u64) -> Duration {
    let mut rng = rand::thread_rng();
    let jitter_range = (base_secs as f64 * 0.15) as i64;
    let jitter = rng.gen_range(-jitter_range..=jitter_range);
    Duration::from_secs((base_secs as i64 + jitter) as u64)
}

// Usage:
let timeout = jittered_timeout(60);  // Returns 51-69 seconds
```

**Timeouts to Jitter:**
| Constant | Base Value | Range After Jitter |
|----------|------------|-------------------|
| `BACKEND_REQUEST_TIMEOUT_SECS` | 60s | 51-69s |
| `TOR_CONTROL_TIMEOUT_SECS` | 15s | 13-17s |
| `header_read_timeout` | 30s | 26-35s |
| `connect_timeout` | 10s | 9-12s |

**Sub-tasks:**
- [ ] Create `jittered_timeout()` helper in fortify-core
- [ ] Apply to backend request timeout
- [ ] Apply to Tor control timeout
- [ ] Apply to header read timeout
- [ ] Apply to connect timeout
- [ ] Document jitter ranges

---

### Task 4: Tor Hidden Service Configuration
**Status:** ⬜ Not Started  
**Estimated Time:** 1 hour  
**Priority:** 🟡 MEDIUM

**Problem:** File-based hidden services don't include all available DoS defense options.

**File to Modify:**
- `crates/fortify-orchestrator/src/tor.rs`

**Current torrc generation (line ~311):**
```rust
"# Fortify mirror: {}\nHiddenServiceDir {}\nHiddenServicePort 80 127.0.0.1:{}\nHiddenServicePoWDefensesEnabled 1\n"
```

**Required Change:**
```rust
"# Fortify mirror: {}
HiddenServiceDir {}
HiddenServicePort 80 127.0.0.1:{}
HiddenServicePoWDefensesEnabled 1
HiddenServiceEnableIntroDoSDefense 1
HiddenServiceMaxStreams 100
HiddenServiceMaxStreamsCloseCircuit 1
"
```

**Options Explained:**
| Option | Value | Purpose |
|--------|-------|---------|
| `HiddenServicePoWDefensesEnabled` | 1 | Already implemented - PoW challenges |
| `HiddenServiceEnableIntroDoSDefense` | 1 | Rate-limit intro point requests |
| `HiddenServiceMaxStreams` | 100 | Max concurrent streams per circuit |
| `HiddenServiceMaxStreamsCloseCircuit` | 1 | Close circuit if MaxStreams exceeded |

**Sub-tasks:**
- [ ] Update file-based torrc generation
- [ ] Add configuration options to OrchestratorConfig
- [ ] Document Tor version requirements
- [ ] Test with Tor 0.4.8+ and 0.4.9+

---

## Verification Checklist

After implementation, verify:

- [ ] `cargo test` passes
- [ ] No new clippy warnings
- [ ] Semaphore correctly limits connections under load
- [ ] 503 returned when threat-tier at capacity (healthy sessions protected)
- [ ] Timeout values vary between requests (log inspection)
- [ ] Tor services created with new config options

---

## 📋 DISCUSSION: Attack Isolation Options (Question C)

**Context:** How should we handle attack traffic once identified? Options below.

### Option 1: Simple 503 Reject
**Description:** Return 503 immediately when threat-tier is at capacity.

| Pros | Cons |
|------|------|
| ✅ Zero resource usage | ❌ Attacker knows they're blocked |
| ✅ Simple implementation | ❌ Attacker can try different circuits |
| ✅ No connection held open | ❌ Fast failure = fast retry |

**Resource Impact:** MINIMAL - Just TCP reset

---

### Option 2: Slow-Drip Response (Tarpit)
**Description:** Accept connection but respond VERY slowly (1 byte per second).

| Pros | Cons |
|------|------|
| ✅ Wastes attacker's connection slots | ❌ Uses our connection slot too |
| ✅ Attacker thinks request is working | ❌ Memory for each tarpitted connection |
| ✅ Can tie up bot resources | ❌ May hit our own connection limits |
| ✅ Already have progressive delay logic | ❌ Needs careful resource accounting |

**Resource Impact:** MEDIUM - 1 connection slot per tarpit + small memory

**Existing Code Reference:**
```rust
// fortify-gate/src/lib.rs line 794
pub fn calculate_delay(&self, failed_attempts: u32) -> u64 {
    match failed_attempts {
        0 => 0, 1 => 2, 2 => 5, 3 => 10, 4 => 20, _ => 30
    }
}
```

---

### Option 3: Full Tarpit (Never Close)
**Description:** Accept connection and never respond or close it.

| Pros | Cons |
|------|------|
| ✅ Maximum attacker resource waste | ❌ Maximum our resource waste |
| ✅ Ties up attacker sockets forever | ❌ Can backfire if attackers have more resources |
| | ❌ Tor may close circuits anyway (timeout) |
| | ❌ Complex state management |

**Resource Impact:** HIGH - Unbounded connections, significant memory

---

### Option 4: Hybrid Approach (RECOMMENDED FOR DISCUSSION)

**Description:** Tiered response based on system load and attacker persistence.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    HYBRID ATTACK HANDLING FLOW                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. First request from unknown session:                                     │
│     └─► Serve static cached CAPTCHA (nearly zero CPU)                       │
│                                                                              │
│  2. Failed CAPTCHA attempt:                                                 │
│     └─► Progressive delay: 2s → 5s → 10s → 20s → 30s cap                   │
│                                                                              │
│  3. Multiple failures (5+) from same circuit:                               │
│     └─► Mark circuit as suspicious, demote to SUSPICIOUS tier               │
│                                                                              │
│  4. System under load (threat capacity > 70%):                              │
│     └─► 503 new threat-tier connections (protect healthy sessions)          │
│                                                                              │
│  5. System under extreme load (threat capacity > 90%):                      │
│     └─► Slow-drip response for existing threat sessions                     │
│     └─► Buying time for healthy sessions to complete                        │
│                                                                              │
│  6. Burned sessions (proven attackers):                                     │
│     └─► Serve static "burned.html" page (no dynamic processing)             │
│     └─► OR slow-drip if resources available                                 │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Resource Budget:**
| Component | Max Connections | Reasoning |
|-----------|-----------------|-----------|
| Healthy pool | 500 | Protected for verified/trusted users |
| Threat pool | 200 | Lower limit, shed these first |
| Tarpit pool | 50 | Small pool for slow-drip responses |
| **Total** | 750 | Hard cap to protect system |

**Key Insight:** The tarpit pool is SEPARATE and SMALL. We only slow-drip a limited number of attackers to avoid exhausting our own resources.

---

### Decision Needed

**Which approach aligns best with your service goals?**

1. **Conservative (Option 1):** Just 503 threat sessions, no resource waste on attackers
2. **Moderate (Option 4):** Hybrid with small tarpit pool (50 connections max)
3. **Aggressive (Option 3):** Full tarpit but risks resource exhaustion

**Resource Concern Trade-off:**
- More tarpits = more attacker resources wasted BUT more of our resources used
- Fewer tarpits = less attacker damage BUT attackers know they're blocked faster

---

## 📋 DISCUSSION: Static CAPTCHA Serving (Question B)

**Current Architecture:**
```
User → Mirror → fortify-http → Gate → CAPTCHA generation → Response
```

**Proposed Optimization:**
```
User → Mirror → fortify-http → Cached CAPTCHA page (from pre-gen pool)
                                    │
                                    └─► /verify → Gate (only this needs processing)
```

**Already Implemented:**
- `CaptchaPoolManager` pre-generates 500 CAPTCHAs
- Pool persists to disk
- CPU-aware generation (pauses at 70%)

**Still Needed:**
- [ ] Serve CAPTCHA HTML directly from fortify-http
- [ ] Embed pre-generated CAPTCHA image in page
- [ ] Only proxy `/verify` to Gate
- [ ] Implement pool rotation for freshness

**Expected Impact:**
- 97% load reduction on Gate
- CAPTCHA page becomes nearly static content
- Attackers cannot exhaust CAPTCHA generation

---

## Related Documents

| Document | Purpose |
|----------|---------|
| [SECURITY-REVIEW-COMPARISON.md](SECURITY-REVIEW-COMPARISON.md) | Full gap analysis |
| [02-PANIC-AUDIT-SPRINT.md](02-PANIC-AUDIT-SPRINT.md) | Related panic prevention work |
| [archive/01-TIMEOUT-STRATEGY-SPRINT.md](archive/01-TIMEOUT-STRATEGY-SPRINT.md) | Completed timeout implementation |
| [Alpha_Review.md](Alpha_Review.md) | Original CAPTCHA optimization proposal |

---

## Dependencies

- Requires tokio sync feature for Semaphore
- Requires rand crate for jitter (already in dependencies)
- Requires Tor 0.4.8+ for IntroDoSDefense

---

*Created based on external security review recommendations - January 23, 2026*
*Updated with discussion notes - January 22, 2026*
