# Sprint 16: Traffic Tier Integration & Settings Activation

**Created**: January 23, 2026  
**Status**: � COMPLETED  
**Branch**: `feature/sprint-16-tier-integration`  
**Priority**: HIGH - Core functionality gap

---

## Implementation Summary

### Completed Changes

1. **Rate Limiter Connected** ✅
   - `GlobalRateLimiter` now accepts `Arc<AdminState>`
   - `get_limit_for_tier()` reads traffic tier from config
   - Tier multipliers applied: Micro=0.5x, Small=1.0x, Medium=2.0x, Large=3.0x, Enterprise=4.0x
   - Unknown/Suspicious tiers always strict (10 req/10s) - not scaled

2. **DDoS Detection Labeled** ✅
   - Added "Coming Soon" label in Control Panel
   - DDoS RPS threshold stored but not yet used for detection

3. **CAPTCHA Pool Labeled** ✅
   - Marked as "⚠️ Restart Required" in Control Panel
   - TUI tier change auto-updates pool_size, min_pool, max_pool

4. **Mirror Settings Labeled** ✅
   - Marked as "⚠️ Restart Required" in Control Panel
   - Note: Orchestrator integration deferred (requires separate AdminState connection)

5. **TUI Tier Selector Added** ✅
   - New "Tier" tab in Settings (first position)
   - Shows current tier, rate limit multiplier, CAPTCHA pool, system requirements
   - Enter key cycles through tiers: Micro → Small → Medium → Large → Enterprise → Micro
   - Auto-updates related CAPTCHA pool settings

6. **Settings Legend Added** ✅
   - Control Panel shows: ✓ Hot Reload | ⚠️ Restart | Coming Soon

---

## Problem Statement

Sprint 15 added the TrafficTier enum and UI selectors, but **the settings are NOT connected to the actual runtime systems**. Currently:

1. **TUI Config** stores settings (pool_size, rate_limit_rpm, etc.)
2. **Control Panel** allows changing traffic tier
3. **BUT** the actual rate limiter uses **hardcoded values** in `GlobalRateLimiter::get_limit_for_tier()`
4. **No mechanism** propagates TUI config changes to running services

### Current Architecture Gap

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        CURRENT STATE (BROKEN)                            │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  TUI Config                    Runtime Services                          │
│  ┌──────────────┐              ┌──────────────────────────┐            │
│  │ traffic_tier │              │ GlobalRateLimiter        │            │
│  │ rate_limit   │ ──── ✗ ────→ │ HARDCODED: 10/100/300    │            │
│  │ pool_size    │   NOT        │                          │            │
│  │ ddos_thresh  │   CONNECTED  │ CaptchaPool              │            │
│  └──────────────┘              │ HARDCODED: 500           │            │
│                                 └──────────────────────────┘            │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Target Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         TARGET STATE (WORKING)                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  TUI Config                    SharedConfig                              │
│  ┌──────────────┐              ┌──────────────────┐                     │
│  │ traffic_tier │              │ Arc<RwLock<>>    │                     │
│  │ rate_limit   │ ──────────→  │ Live Config      │                     │
│  │ pool_size    │              │                  │                     │
│  └──────────────┘              └────────┬─────────┘                     │
│                                          │                               │
│                    ┌─────────────────────┼─────────────────────┐        │
│                    ▼                     ▼                     ▼        │
│           ┌──────────────┐      ┌──────────────┐      ┌──────────────┐ │
│           │ RateLimiter  │      │ CaptchaPool  │      │ Orchestrator │ │
│           │ reads config │      │ reads config │      │ reads config │ │
│           └──────────────┘      └──────────────┘      └──────────────┘ │
│                                                                          │
│  Settings Changes: HOT RELOAD (marked) vs RESTART REQUIRED              │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Gap Analysis

### 1. Rate Limiter - HARDCODED (lib.rs:59-64)

**Current Code**:
```rust
fn get_limit_for_tier(&self, tier: TrustTier) -> usize {
    match tier {
        TrustTier::Trusted => 300,   // HARDCODED!
        TrustTier::Verified => 100,  // HARDCODED!
        TrustTier::Unknown => 10,    // HARDCODED!
    }
}
```

**Problem**: These values should come from TrafficTier config, not be hardcoded.

**Fix Required**:
- [ ] Pass `Arc<AdminState>` or shared config to `GlobalRateLimiter`
- [ ] Read rate limits from config at check time (hot-reload capable)
- [ ] Scale Trust Tier limits based on Traffic Tier selection

### 2. CAPTCHA Pool - PARTIALLY CONNECTED (admin.rs)

**Current Code**:
```rust
pub fn apply_traffic_tier(&self, tier: TrafficTier) {
    inner.captcha_pool_config.pool_size = tier.pool_size();
    // ... updates AdminState but NOT the actual CaptchaPool
}
```

**Problem**: Updates config struct but doesn't trigger pool resize.

**Fix Required**:
- [ ] Add method to resize CAPTCHA pool at runtime
- [ ] Mark pool_size as **REQUIRES RESTART** if resize not feasible
- [ ] Or implement hot-resize of pool background task

### 3. Mirror Counts - NOT CONNECTED

**Current**: TUI stores min_mirrors/max_mirrors but Orchestrator doesn't read them.

**Fix Required**:
- [ ] Orchestrator reads mirror config from shared state
- [ ] Implement max_mirrors enforcement
- [ ] Mark as **REQUIRES RESTART** for min_mirrors changes

### 4. DDoS Threshold - NOT USED

**Current**: `ddos_rps_threshold` stored but no DDoS detection system.

**Fix Required**:
- [ ] Implement DDoS detection using threshold
- [ ] Or remove/hide the setting if not implemented

### 5. Ban Thresholds - PARTIALLY CONNECTED

**Current**: `temp_ban_minutes` and `perm_ban_threshold` stored but may not be read.

**Fix Required**:
- [ ] Verify behavioral analysis reads these values
- [ ] Add hot-reload or mark as **REQUIRES RESTART**

---

## Settings Classification

### 🔄 HOT RELOAD (Changes take effect immediately)

| Setting | Component | Implementation Status |
|---------|-----------|----------------------|
| Rate Limits (RPM) | GlobalRateLimiter | ❌ Hardcoded - needs fix |
| DDoS Threshold | DDoS Detector | ❌ Not implemented |
| Temp Ban Duration | BehavioralAnalyzer | ⚠️ Verify connection |
| Perm Ban Threshold | BehavioralAnalyzer | ⚠️ Verify connection |
| Branding Colors | Gate/HTTP | ✅ Working |
| Service Name | Gate/HTTP | ✅ Working |

### 🔁 REQUIRES RESTART (Changes need service restart)

| Setting | Component | Reason |
|---------|-----------|--------|
| CAPTCHA Pool Size | CaptchaPool | Pool allocated at startup |
| Min/Max Pool Size | CaptchaPool | Emergency thresholds |
| Mirror Count Changes | Orchestrator | Tor circuits established at start |
| Network Bindings | HTTP/Gate | Socket bindings |
| Backend Address | ProxyHandler | Connection pool |
| Vanguards Settings | Tor | Tor configuration |

---

## Implementation Tasks

### Phase 1: Rate Limiter Integration (HIGH PRIORITY)

**File**: `fortify-http/src/lib.rs`

1. [ ] Add `Arc<AdminState>` parameter to `GlobalRateLimiter::new()`
2. [ ] Modify `get_limit_for_tier()` to read from config
3. [ ] Create scaling formula: `base_limit * traffic_tier_multiplier`
4. [ ] Test rate limit changes propagate immediately

**Proposed Scaling**:
```
Trust Tier Base Limits (per 10s window):
  - Unknown: 10 (fixed - always strict for new visitors)
  - Verified: TrafficTier.rate_limit_rpm / 6 (convert RPM to per-10s)
  - Trusted: TrafficTier.rate_limit_rpm / 3 (2x Verified)

Example for Medium Tier (120 RPM):
  - Unknown: 10/10s
  - Verified: 20/10s
  - Trusted: 40/10s
```

### Phase 2: CAPTCHA Pool Integration

**File**: `fortify-gate/src/captcha/pool.rs` (if exists) or equivalent

1. [ ] Add pool resize capability OR mark as restart-required
2. [ ] Update min/max thresholds at runtime if possible
3. [ ] Add UI label: "⚠️ Requires Restart"

### Phase 3: Mirror Integration

**File**: `fortify-orchestrator/src/lib.rs`

1. [ ] Read min/max mirrors from shared config
2. [ ] Enforce max_mirrors limit
3. [ ] Add UI label: "⚠️ Requires Restart"

### Phase 4: UI Labels for Restart Requirements

**Files**: `fortify-http/src/admin.rs`, `fortify-tui/src/ui/settings.rs`

1. [ ] Add warning icons/text to settings that require restart
2. [ ] Group settings by "Hot Reload" vs "Requires Restart"
3. [ ] Add confirmation dialog for restart-required changes
4. [ ] Show current vs pending values for restart-required settings

### Phase 5: Tier Value Adjustments

**Files**: `fortify-tui/src/config.rs`, `fortify-http/src/admin.rs`

Adjust Large and Enterprise to more Tor-realistic values:

| Tier | Current Pool | New Pool | Current DDoS | New DDoS | Current Mirrors | New Mirrors |
|------|--------------|----------|--------------|----------|-----------------|-------------|
| Large | 5,000 | 3,000 | 2,000 | 1,000 | 5-20 | 4-12 |
| Enterprise | 10,000 | 5,000 | 10,000 | 3,000 | 10-50 | 6-20 |

Add system requirements labels:

Add system requirements labels (includes ~1GB OS overhead for Ubuntu Server):

| Tier | Min CPU | Rec CPU | Min RAM | Rec RAM | Min Disk |
|------|---------|---------|---------|---------|----------|
| Micro | 1 core | 2 cores | 2GB | 2GB | 1GB |
| Small | 2 cores | 4 cores | 2GB | 3GB | 2GB |
| Medium | 2 cores | 4 cores | 3GB | 4GB | 3GB |
| Large | 4 cores | 8 cores | 5GB | 8GB | 6GB |
| Enterprise | 4 cores | 8 cores | 8GB | 16GB | 10GB |

> **Note:** RAM values include Ubuntu Server OS overhead (~1GB). Bare metal or minimal OS may use less.

---

## Questions for Implementation

### Q1: Rate Limit Scaling Strategy

**Options**:
1. **Multiply base limits by tier** (Micro=0.5x, Small=1x, Medium=2x, Large=4x, Enterprise=8x)
2. **Use absolute values from config** (TrafficTier.rate_limit_rpm directly)
3. **Hybrid**: Base limits stay, but Trust Tier scaling adjusts

**DECISION**: Option 1 - Scale base values with tier multipliers.

Implementation:
```rust
// Base limits (per 10s window):
const BASE_UNKNOWN: usize = 10;   // Always strict for new visitors
const BASE_VERIFIED: usize = 100;
const BASE_TRUSTED: usize = 300;

// Tier multipliers:
// Micro=0.5x, Small=1.0x, Medium=2.0x, Large=3.0x, Enterprise=4.0x
```

### Q2: CAPTCHA Pool Resize

**Options**:
1. **Hot resize**: Complex, need to manage generation threads
2. **Restart required**: Simple, just mark the setting
3. **Hybrid**: Allow increase (add more), require restart for decrease

**DECISION**: Start with Option 2 (Restart required), plan for full hot-reload in Phase 4.

**Future Hot-Reload Juggling System (Phase 4)**:

To enable seamless hot-reload while maintaining service uptime:

1. **Session Juggling Architecture**:
   - When a restart-required setting changes, don't restart immediately
   - Take half the nodes/mirrors offline
   - Route all sessions to remaining active nodes
   - Apply changes to offline nodes and bring them back online
   - Route sessions to updated nodes
   - Repeat for remaining nodes
   - Result: Zero-downtime hot-reload

2. **Implementation Requirements**:
   - [ ] Session routing awareness of node states
   - [ ] Graceful node shutdown with session drain
   - [ ] Staggered restart orchestration
   - [ ] Health check before accepting sessions

3. **Settings Classification**:
   - **Always Hot-Reload**: Rate limits, ban thresholds, branding
   - **Juggle-able**: CAPTCHA pool, mirror counts, node config
   - **True Restart**: Network bindings, Tor settings

### Q3: DDoS Detection

**Current**: Not implemented. Options:
1. **Implement full DDoS detector** using `ddos_rps_threshold`
2. **Hide setting** until implemented
3. **Label as "Coming Soon"**

**DECISION**: Option 3 - Label as "Coming Soon (not yet active)" in UI.

Future implementation will use the threshold for automatic DDoS mode activation.

### Q4: Restart UI Behavior

**Options**:
1. **Extend AdminState** - Already shared, add more fields
2. **New SharedConfig type** - Cleaner but more refactoring
3. **Environment reload** - Re-read config file periodically

**DECISION**: 
- **Current Sprint**: Show warning only - "This setting requires a service restart to take effect"
- **Future (Phase 4)**: Add "Apply & Restart" button with session juggling so sessions continue unaffected during hot-reload

**UI Implementation**:
```html
<!-- For restart-required settings -->
<span style="color: var(--crimson);">⚠️ Requires Restart</span>
<p style="font-size: 0.8em;">Changes will take effect after service restart.</p>

<!-- Future: With juggling -->
<button>Apply Now (will juggle sessions)</button>
```

---

## Testing Plan

### Unit Tests
- [ ] Rate limiter respects config values
- [ ] Traffic tier changes propagate to config
- [ ] Settings classification correct

### Integration Tests
- [ ] Change tier in Control Panel → rate limits update
- [ ] Restart-required settings show warning
- [ ] Config persists across restarts

### Manual Testing
- [ ] TUI tier selector works
- [ ] Control Panel tier selector works
- [ ] Rate limiting behavior matches tier
- [ ] Labels show correctly

---

## Acceptance Criteria

1. ✅ Selecting a Traffic Tier updates ALL related settings
2. ✅ Rate limiter uses config values, not hardcoded
3. ✅ Settings are labeled: "Hot Reload" or "Requires Restart"
4. ✅ Large/Enterprise tiers have realistic Tor-compatible values
5. ✅ System requirements shown for each tier
6. ✅ Restart-required changes show warning before applying
7. ✅ All settings persist to config file
8. ✅ Services read settings from shared config at runtime

---

## Files to Modify

| File | Changes |
|------|---------|
| `fortify-http/src/lib.rs` | Rate limiter reads from config |
| `fortify-http/src/admin.rs` | Add system requirements labels, update tier values |
| `fortify-tui/src/config.rs` | Update tier values, add requirements |
| `fortify-tui/src/ui/settings.rs` | Add restart-required labels |
| `Deploy-Scripts/SETTINGS.md` | Update with accurate tier values |
| `Deploy-Scripts/deploy-*.sh` | Update tier values |

---

## Timeline Estimate

| Phase | Effort | Priority |
|-------|--------|----------|
| Phase 1: Rate Limiter | 2-3 hours | HIGH |
| Phase 2: CAPTCHA Pool | 1 hour (label only) | MEDIUM |
| Phase 3: Mirror Integration | 1 hour | MEDIUM |
| Phase 4: UI Labels | 2 hours | HIGH |
| Phase 5: Tier Adjustments | 1 hour | HIGH |
| Testing | 2 hours | HIGH |

**Total**: ~10 hours

---

## Notes

- This sprint addresses a **core functionality gap** where settings appear to work but don't affect runtime
- Focus on making existing settings work before adding new ones
- "Requires Restart" labeling is acceptable for complex settings
- Hot-reload for rate limits is critical for operational flexibility
