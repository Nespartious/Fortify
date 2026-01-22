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

### Current Session Flow (Baseline)

Before discussing options, here's how sessions currently traverse the system:

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    CURRENT SESSION FLOW TRAVERSAL                                │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  NEW USER (No Token):                                                           │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  1. Request arrives at Mirror (.onion)                                   │   │
│  │  2. fortify-http checks: No token/invalid token                          │   │
│  │  3. TrustTier = Unknown → requires_gate() = TRUE                        │   │
│  │  4. Proxy to Gate service                                                │   │
│  │  5. Gate serves CAPTCHA page (currently dynamically, want static)       │   │
│  │  6. User solves CAPTCHA → POST /verify                                  │   │
│  │  7. Gate validates → Issues VERIFIED token                              │   │
│  │  8. User redirected back with session cookie                            │   │
│  │  9. Next request: token valid → TrustTier = Verified                    │   │
│  │  10. Route to Healthy Nodes → See real site                              │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
│  VERIFIED USER (Good behavior):                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  1. Request with valid VERIFIED token                                    │   │
│  │  2. requires_gate() = FALSE → Route to Healthy Nodes                    │   │
│  │  3. Behavioral analysis runs on each request                            │   │
│  │  4. Clean behavior → May promote to TRUSTED                             │   │
│  │  5. Continue serving real site                                          │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
│  VERIFIED USER (Bad behavior → Demotion):                                       │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  1. Request with valid VERIFIED token                                    │   │
│  │  2. Behavioral analysis detects: path enumeration, bot UA, etc.         │   │
│  │  3. Violation count exceeds threshold                                   │   │
│  │  4. DEMOTE: TrustTier → SUSPICIOUS                                      │   │
│  │  5. Set fortify_demoted=1 cookie                                        │   │
│  │  6. Redirect to Gate                                                    │   │
│  │  7. Gate sees demoted=1 → HARD difficulty, 2 CAPTCHAs required         │   │
│  │  8. User solves both → Re-issued as VERIFIED                            │   │
│  │  9. Back to healthy path (with fresh token)                             │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
│  BURNED USER (Proven attacker):                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  1. Request with BURNED token OR admin-burned session ID                │   │
│  │  2. serve_killed_session_page() → Static burned.html                    │   │
│  │  3. No further processing                                               │   │
│  │  4. NO RECOVERY PATH (permanent)                                        │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Key Insight:** The Gate (CAPTCHA) is the bottleneck. Attackers exhaust Gate capacity → legitimate new users can't get verified → service appears down to new visitors.

---

### How Each Option Changes Session Flow

#### Option 1: Simple 503 Reject

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    OPTION 1: 503 REJECT FLOW                                     │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  SYSTEM STATE: Threat capacity at 70%+                                          │
│                                                                                  │
│  NEW USER arrives:                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  1. Request → TrustTier = Unknown → requires_gate() = TRUE              │   │
│  │  2. Check: threat_pool_usage > 70%?                                     │   │
│  │     YES → Return 503 + static "busy.html"                               │   │
│  │     NO  → Continue to Gate as normal                                    │   │
│  │                                                                          │   │
│  │  3. User sees: "Service is busy. Retry in 30 seconds."                  │   │
│  │     <meta http-equiv="refresh" content="30"> (auto-refresh, no JS)      │   │
│  │                                                                          │   │
│  │  4. User refreshes → Same check → May get through if load drops         │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
│  VERIFIED/TRUSTED USER:                                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  ✅ UNAFFECTED - They never touch threat pool                           │   │
│  │  Route directly to Healthy Nodes (separate capacity pool)               │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
│  FALSE POSITIVE IMPACT:                                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  - Legitimate new user during attack: Sees 503, waits, tries again      │   │
│  │  - Recovery: Automatic on retry when load drops                         │   │
│  │  - No permanent damage, just delayed access                             │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Session Flow Changes:**
- NEW path adds capacity check before Gate proxy
- VERIFIED/TRUSTED paths unchanged
- DEMOTED users also hit the 503 check (they're in threat tier)

**Problem:** Demoted legitimate users (false positive demotions) must wait out the 503 period even though they have a history.

---

#### Option 2: Slow-Drip Response (Tarpit)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    OPTION 2: SLOW-DRIP FLOW                                      │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  SYSTEM STATE: Threat capacity at 70%+                                          │
│                                                                                  │
│  NEW USER arrives during overload:                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  1. Request → TrustTier = Unknown → requires_gate() = TRUE              │   │
│  │  2. Check: threat_pool_usage > 70%?                                     │   │
│  │     YES → Route to TARPIT handler instead of Gate                       │   │
│  │                                                                          │   │
│  │  3. Tarpit handler:                                                     │   │
│  │     - Accept TCP connection (uses 1 slot from tarpit pool)              │   │
│  │     - Send HTTP headers very slowly (1 byte per second)                 │   │
│  │     - Never complete response                                           │   │
│  │     - Connection sits open for minutes                                  │   │
│  │                                                                          │   │
│  │  4. Attacker's client waits... and waits... (resource tied up)          │   │
│  │  5. Eventually times out on attacker side                               │   │
│  │  6. Our tarpit slot freed                                               │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
│  VERIFIED/TRUSTED USER:                                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  ✅ UNAFFECTED - They never touch threat pool                           │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
│  FALSE POSITIVE IMPACT:                                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  ⚠️ SEVERE - Legitimate new user gets tarpitted!                        │   │
│  │  - Their browser hangs waiting for response                             │   │
│  │  - No error message, just loading spinner                               │   │
│  │  - Must manually cancel and retry                                       │   │
│  │  - Much worse UX than 503                                               │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
│  RESOURCE ACCOUNTING:                                                           │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  - Each tarpit connection: ~1KB memory + 1 file descriptor              │   │
│  │  - Max 50 concurrent tarpits = 50KB + 50 FDs                            │   │
│  │  - Attacker with 1000 bots: 950 get 503, 50 get tarpitted               │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Session Flow Changes:**
- NEW path can diverge to tarpit instead of Gate
- Creates hanging connections for unknowns
- VERIFIED/TRUSTED unchanged

**Problem:** Legitimate new users get terrible UX (browser hangs indefinitely).

---

#### Option 3: Full Tarpit (Never Close)

Same as Option 2 but MORE aggressive:
- Never sends ANY bytes
- Keeps connection open until client gives up or system reboot
- Maximum resource waste on BOTH sides

**Problem:** Same as Option 2 but even worse for false positives. Also risks running out of file descriptors if attackers open more connections than we can tarpit.

---

#### Option 4: Hybrid Approach (Refined)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    OPTION 4: HYBRID FLOW (RECOMMENDED)                           │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  KEY INSIGHT: Only tarpit REPEAT offenders, not first-timers                    │
│                                                                                  │
│  NEW USER (first visit, no history):                                            │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  1. Request → TrustTier = Unknown                                       │   │
│  │  2. Check: threat_pool_usage > 70%?                                     │   │
│  │     YES → Return 503 + "busy.html" with auto-refresh                    │   │
│  │     NO  → Serve static cached CAPTCHA page                              │   │
│  │                                                                          │   │
│  │  NOTE: First-timers ALWAYS get 503, never tarpit                        │   │
│  │        They haven't proven malicious yet                                │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
│  SUSPICIOUS USER (demoted, failed CAPTCHAs):                                    │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  1. Request → TrustTier = Suspicious (has history of bad behavior)      │   │
│  │  2. Check: failed_captcha_count > 3 AND threat_pool_usage > 80%?        │   │
│  │     YES → Candidate for tarpit (if slots available)                     │   │
│  │     NO  → Normal Gate flow with HARD difficulty                         │   │
│  │                                                                          │   │
│  │  3. If tarpit slot available:                                           │   │
│  │     - Slow-drip response (tie up their resources)                       │   │
│  │  4. If no tarpit slots:                                                 │   │
│  │     - 503 reject                                                        │   │
│  │                                                                          │   │
│  │  NOTE: Only proven bad actors get tarpitted                             │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
│  BURNED USER:                                                                   │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  1. Request → TrustTier = Burned                                        │   │
│  │  2. ALWAYS tarpit if slot available (they're confirmed bad)             │   │
│  │  3. If no tarpit slots → Static burned.html                             │   │
│  │                                                                          │   │
│  │  NOTE: Burned users are ideal tarpit candidates                         │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
│  VERIFIED/TRUSTED USER:                                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  ✅ COMPLETELY UNAFFECTED                                                │   │
│  │  - Separate capacity pool (healthy nodes)                               │   │
│  │  - No 503, no tarpit, no delays                                         │   │
│  │  - Only checked under EXTREME system-wide emergency (99%+ overall)      │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
│  FALSE POSITIVE PROTECTION:                                                     │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  - First-time visitors: 503 with friendly message, auto-retry           │   │
│  │  - Demoted but legitimate: Gets 503, not tarpit (< 3 CAPTCHA fails)     │   │
│  │  - Only tarpit: 3+ CAPTCHA failures OR burned status                    │   │
│  │  - Recovery: Solve CAPTCHA → Back to healthy path                       │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Session Flow Changes:**

| User Type | Normal Load | High Load (70%+) | Extreme (90%+) |
|-----------|-------------|------------------|----------------|
| **Unknown (new)** | CAPTCHA page | 503 + retry | 503 + retry |
| **Suspicious (demoted)** | HARD CAPTCHA x2 | 503 OR tarpit* | Tarpit if available |
| **Burned** | burned.html | Tarpit if available | Tarpit if available |
| **Verified** | Real site | Real site | Real site |
| **Trusted** | Real site | Real site | Real site (even at 99%) |

*Tarpit only if: failed_captcha_count >= 3

---

### Critical Question: What Happens to Demoted Legitimate Users?

This is where I need your input. Consider this scenario:

**Scenario:** Legitimate user gets demoted due to borderline behavior (e.g., refreshed page 10 times quickly looking for updates).

| Option | What They Experience | Recovery Path |
|--------|---------------------|---------------|
| **Option 1 (503)** | "Service busy" page, auto-refresh in 30s | Wait, retry, solve CAPTCHA when load drops |
| **Option 2 (Tarpit)** | Browser hangs forever | Must close tab, come back later |
| **Option 4 (Hybrid)** | Same as Option 1 (protected by fail count) | Wait, retry, solve CAPTCHA |

With Option 4, a demoted user who hasn't failed multiple CAPTCHAs is NOT tarpitted. They get the friendly 503 experience.

---

### My Recommendation

**Option 4 (Hybrid)** because:

1. **First-timers protected** - Never tarpitted, just politely asked to retry
2. **False positive demotions protected** - Need 3+ CAPTCHA failures before tarpit
3. **Proven attackers punished** - Burned users always tarpit candidates
4. **Resource controlled** - Fixed tarpit pool (50 max)
5. **Verified/Trusted untouched** - Your primary goal achieved

**Do you want me to refine Option 4 further, or do you have concerns about specific scenarios?**

---

## 📋 DISCUSSION: Busy Page / 503 Page Design (Question B)

**Constraint:** NO JAVASCRIPT. Pure HTML/CSS only.

### Available Mechanisms for Auto-Retry (No JS)

#### 1. Meta Refresh Tag
```html
<meta http-equiv="refresh" content="30;url=/Fortify">
```
- Browser auto-refreshes after 30 seconds
- Works in ALL browsers including Tor Browser Safest mode
- Simple, reliable, no JS

#### 2. Retry-After HTTP Header
```http
HTTP/1.1 503 Service Unavailable
Retry-After: 30
Content-Type: text/html
```
- Tells well-behaved clients when to retry
- Not all browsers respect this automatically
- Bots/scrapers may ignore

#### 3. Pure CSS Countdown (Visual Only)
```css
/* CSS animation for visual countdown - no actual timer */
@keyframes countdown {
  0% { content: "30"; }
  3.33% { content: "29"; }
  /* ... */
  100% { content: "0"; }
}
.timer::before {
  animation: countdown 30s linear forwards;
}
```
- Visual countdown that matches meta refresh
- Gives user feedback that page will auto-refresh
- Purely decorative, meta refresh does the real work

### Proposed 503 Page Design

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta http-equiv="refresh" content="30;url=/Fortify">
    <title>Service Busy - Please Wait</title>
    <style>
        body { 
            background: #1a1a2e; 
            color: #e0e0e0; 
            font-family: monospace; 
            text-align: center;
            padding: 50px;
        }
        .container {
            max-width: 500px;
            margin: 0 auto;
            border: 1px solid #6B46C1;
            padding: 40px;
            border-radius: 8px;
        }
        h1 { color: #6B46C1; }
        .status { 
            font-size: 4em; 
            color: #f59e0b; 
        }
        .message { 
            margin: 20px 0; 
            line-height: 1.6;
        }
        .retry-link {
            display: inline-block;
            margin-top: 20px;
            padding: 10px 30px;
            background: #6B46C1;
            color: white;
            text-decoration: none;
            border-radius: 4px;
        }
        .auto-note {
            margin-top: 30px;
            color: #888;
            font-size: 0.9em;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="status">503</div>
        <h1>Service Temporarily Busy</h1>
        <p class="message">
            The service is experiencing high demand.<br>
            Your request could not be processed at this time.
        </p>
        <a href="/Fortify" class="retry-link">Retry Now</a>
        <p class="auto-note">
            This page will automatically retry in 30 seconds.
        </p>
    </div>
</body>
</html>
```

### Page Variations Needed

| Situation | Page | Auto-Refresh | Manual Retry |
|-----------|------|--------------|--------------|
| Threat pool full (new user) | 503-busy.html | 30s | Yes |
| Threat pool full (demoted user) | 503-busy.html | 30s | Yes |
| Healthy pool full (verified - rare) | 503-overload.html | 5s | Yes |
| Burned session | burned.html | NO | NO |
| Tarpitted session | (slow-drip response, no page) | N/A | N/A |

### Question B Summary

**Recommended approach:**
1. `<meta http-equiv="refresh" content="30">` for auto-retry
2. Manual "Retry Now" link for impatient users
3. Clear message explaining the situation
4. No JavaScript required
5. Works in Tor Browser Safest mode

**Does this address your Question B concerns? Any adjustments needed?**

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
