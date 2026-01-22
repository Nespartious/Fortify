# Sprint: Security Status Degradation Bug Fix

**Sprint ID:** BUG-001  
**Priority:** 🔴 HIGH (User-Reported Bug)  
**GitHub Issue:** [#18](https://github.com/Nespartious/Fortify/issues/18)  
**Estimated Effort:** 1-2 days  
**Status:** ⬜ Not Started  
**Created:** January 22, 2026

---

## Problem Statement

Security status does not degrade from "Attack" once it switches to that level. The status persists indefinitely even after the attack conditions subside.

### User Report
- **Symptom:** Security status shows "Attack" forever once triggered
- **Expected:** Status should degrade back to "Healthy"/"Clear" over time
- **Impact:** Operators cannot see when attack has subsided; false positive sustained alert

---

## Root Cause Analysis

### Location
- **File:** `crates/fortify-tui/src/logging.rs`
- **Struct:** `SecurityStatus`
- **Method:** `compute_level()` (lines 275-297)

### Current Behavior

```rust
pub fn compute_level(&mut self) {
    self.maybe_swap_buckets();

    let sessions_per_min = self.new_sessions_per_minute();
    let unverified_per_min = self.unverified_requests_per_minute();
    let failed_captcha = self.failed_captcha_attempts;
    let has_suspicious_flags = !self.suspicious_flags.is_empty();

    // Thresholds (these could be configurable)
    self.level = if sessions_per_min > 100 || unverified_per_min > 500 || failed_captcha > 20 {
        SecurityLevel::Attack
    } else if sessions_per_min > 60 || unverified_per_min > 300 || ... {
        SecurityLevel::Warning
    } else if ... {
        SecurityLevel::Suspicious
    } else if ... {
        SecurityLevel::Elevated
    } else if sessions_per_min > 0 || unverified_per_min > 0 {
        SecurityLevel::Normal
    } else {
        SecurityLevel::Clear
    };
}
```

### Issue #1: Buckets Not Swapping Properly
The `maybe_swap_buckets()` function requires 30 seconds to pass, but if `compute_level()` isn't called regularly, buckets may never swap.

### Issue #2: Attack Counters Never Reset
`failed_captcha_attempts` and `resolved_sessions` only decay in `maybe_swap_buckets()`:
```rust
self.resolved_sessions = self.resolved_sessions.saturating_sub(5);
self.failed_captcha_attempts = self.failed_captcha_attempts.saturating_sub(2);
```

But if attack triggers `failed_captcha > 20`, and decay is only `-2 per 30s`, it takes:
- `20 / 2 * 30s = 5 minutes` minimum to drop below threshold
- This assumes no new failures during that time

### Issue #3: No Hysteresis
Status immediately jumps to "Attack" when thresholds crossed, but should require sustained conditions and/or have a "recovery period" before downgrading.

---

## Proposed Solution

### 1. Add Regular Timer-Based Bucket Swap
```rust
// In App::tick() - called every second
self.security_status.tick();

// In SecurityStatus
pub fn tick(&mut self) {
    self.maybe_swap_buckets();
    self.decay_counters();
}

fn decay_counters(&mut self) {
    // Faster decay for stuck counters
    if self.level != SecurityLevel::Attack {
        self.failed_captcha_attempts = self.failed_captcha_attempts.saturating_sub(1);
    }
}
```

### 2. Add Attack Duration Tracking
```rust
pub struct SecurityStatus {
    // ... existing fields ...
    /// When current level was set
    pub level_since: std::time::Instant,
    /// Minimum time before level can downgrade
    pub level_hold_seconds: u64,
}
```

### 3. Implement Hysteresis in `compute_level()`
```rust
pub fn compute_level(&mut self) {
    self.maybe_swap_buckets();
    
    let candidate_level = self.calculate_candidate_level();
    
    // Don't immediately downgrade - require sustained improvement
    if candidate_level < self.level {
        let hold_time = match self.level {
            SecurityLevel::Attack => 120,  // 2 min hold
            SecurityLevel::Warning => 60,  // 1 min hold
            _ => 30,                       // 30s hold
        };
        
        if self.level_since.elapsed().as_secs() >= hold_time {
            self.level = candidate_level;
            self.level_since = std::time::Instant::now();
        }
    } else if candidate_level > self.level {
        // Immediate upgrade allowed
        self.level = candidate_level;
        self.level_since = std::time::Instant::now();
    }
}
```

---

## Implementation Tasks

### Task 1: Add `tick()` Method
**Status:** ⬜ Not Started  
**Estimated Time:** 30 minutes

- [ ] Add `tick()` method to `SecurityStatus`
- [ ] Call `tick()` from `App::tick()` (every 1 second)
- [ ] Add decay logic for `failed_captcha_attempts`

### Task 2: Add Level Duration Tracking
**Status:** ⬜ Not Started  
**Estimated Time:** 30 minutes

- [ ] Add `level_since: Instant` field
- [ ] Update `Default` and `new()` implementations
- [ ] Update level changes to reset `level_since`

### Task 3: Implement Hysteresis Logic
**Status:** ⬜ Not Started  
**Estimated Time:** 1 hour

- [ ] Create `calculate_candidate_level()` method (pure calculation)
- [ ] Add hysteresis in `compute_level()`
- [ ] Make hold times configurable via `ThresholdConfig`

### Task 4: Testing
**Status:** ⬜ Not Started  
**Estimated Time:** 1 hour

- [ ] Add unit tests for level degradation
- [ ] Add test for hysteresis timing
- [ ] Manual testing with simulated attack

### Task 5: Documentation
**Status:** ⬜ Not Started  
**Estimated Time:** 30 minutes

- [ ] Document hysteresis behavior in operator guide
- [ ] Add logging for level transitions
- [ ] Close GitHub issue #18

---

## Test Plan

### Unit Tests
```rust
#[test]
fn test_level_degrades_after_attack() {
    let mut status = SecurityStatus::new();
    
    // Simulate attack conditions
    for _ in 0..25 {
        status.record_failed_captcha();
    }
    status.compute_level();
    assert_eq!(status.level, SecurityLevel::Attack);
    
    // Simulate time passing with no new events
    // (mock Instant or use test helper)
    status.simulate_time_passing(Duration::from_secs(180));
    
    // Should have degraded
    assert!(status.level < SecurityLevel::Attack);
}
```

### Manual Test
1. Start Fortify in development mode
2. Generate 25+ failed CAPTCHA attempts quickly
3. Observe status change to "Attack"
4. Stop generating failures
5. Wait 2-3 minutes
6. Verify status degrades to "Warning" then lower

---

## Acceptance Criteria

- [ ] Security status degrades within 5 minutes of attack conditions subsiding
- [ ] Hysteresis prevents rapid oscillation between levels
- [ ] Hold times are configurable
- [ ] Level transitions are logged
- [ ] Unit tests pass
- [ ] GitHub issue #18 closed with fix reference
