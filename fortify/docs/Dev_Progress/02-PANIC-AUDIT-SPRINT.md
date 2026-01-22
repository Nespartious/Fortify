# Sprint: Panic Audit & Error Handling

**Sprint ID:** BETA-002  
**Priority:** 🔴 CRITICAL (Beta Blocker)  
**Estimated Effort:** 3-5 days  
**Status:** ⬜ Not Started  
**Created:** January 22, 2026

---

## Objective

Systematically audit and replace all unsafe panic paths in network-facing code with proper error handling. Prevent DoS attacks via intentional panic triggers.

## Success Criteria

- [ ] Zero `unwrap()` calls on untrusted input in production code
- [ ] Lock poisoning handled gracefully (no cascading failures)
- [ ] Fuzzing infrastructure operational
- [ ] Clippy lints enforced: `#![deny(clippy::unwrap_used)]`
- [ ] Service remains running under malformed/malicious input

---

## Risk Categories

| Priority | Category | Risk Level | Action |
|----------|----------|------------|--------|
| 🔴 CRITICAL | Network input (headers, cookies, body) | Attacker-controlled | Must fix all |
| 🔴 CRITICAL | Token/session deserialization | Attacker-controlled | Must fix all |
| 🟡 HIGH | Lock operations (Mutex/RwLock) | Can cascade | Handle poisoning |
| 🟢 LOW | Startup/config parsing | Validated early | Acceptable |
| ⚪ SAFE | Test code | Not production | No action |

---

## Implementation Tasks

### Task 1: Comprehensive Panic Audit
**Status:** ⬜ Not Started  
**Estimated Time:** 1 hour

**Commands to run:**
```bash
cd /home/shadowbox/Fortify/Fortify/fortify

# Find all unwrap() calls
grep -rn "\.unwrap()" crates/fortify-*/src/ \
  | grep -v "/test" | grep -v "_test.rs" > /tmp/unwrap_audit.txt

# Find all expect() calls
grep -rn "\.expect(" crates/fortify-*/src/ \
  | grep -v "/test" | grep -v "_test.rs" > /tmp/expect_audit.txt

# Find all panic!() calls
grep -rn "panic!" crates/fortify-*/src/ \
  | grep -v "/test" | grep -v "_test.rs" > /tmp/panic_audit.txt

# Count totals
wc -l /tmp/unwrap_audit.txt /tmp/expect_audit.txt /tmp/panic_audit.txt
```

**Deliverables:**
- [ ] Create audit files with all panic points
- [ ] Categorize each by risk level (CRITICAL/HIGH/LOW/SAFE)
- [ ] Create tracking spreadsheet with columns:
  - File:Line
  - Code Context
  - Risk Level
  - Input Source (network/user/config/internal)
  - Fix Priority (P0/P1/P2)
  - Status

---

### Task 2: Fix HTTP Header Parsing
**Status:** ⬜ Not Started  
**Estimated Time:** 2 hours  
**Files:**
- `crates/fortify-http/src/middleware.rs`
- `crates/fortify-http/src/routing.rs`
- `crates/fortify-gate/src/server.rs`

**Vulnerable Patterns to Find:**
```rust
// ❌ VULNERABLE - Can panic if header missing
let cookie = headers.get("cookie").unwrap();

// ❌ VULNERABLE - Can panic if header invalid UTF-8
let session = cookie.to_str().unwrap();

// ❌ VULNERABLE - Index panic if no '=' found
let parts: Vec<&str> = header.split('=').collect();
let value = parts[1];
```

**Safe Replacements:**
```rust
// ✅ SAFE - Returns error if header missing
let cookie = headers
    .get("cookie")
    .ok_or(HttpError::MissingCookie)?;

// ✅ SAFE - Returns error if invalid UTF-8
let session = cookie
    .to_str()
    .map_err(|_| HttpError::InvalidCookie)?;

// ✅ SAFE - Use split_once()
let (key, value) = header
    .split_once('=')
    .ok_or(HttpError::MalformedHeader)?;
```

**Sub-tasks:**
- [ ] Search for `.get().unwrap()` pattern on headers
- [ ] Search for `.to_str().unwrap()` on header values
- [ ] Search for array indexing on split results
- [ ] Replace all with safe error handling
- [ ] Add custom error types if needed

**Test Commands:**
```bash
# Test with missing headers
curl -X POST http://localhost:8082 -H "Content-Length: 0"

# Test with invalid UTF-8 in header
curl -X POST http://localhost:8082 -H "X-Session: $(printf '\xff\xfe')"

# Test with malformed cookie
curl http://localhost:8082 -H "Cookie: no_equals_sign"
```

---

### Task 3: Fix Token/Session Deserialization
**Status:** ⬜ Not Started  
**Estimated Time:** 2 hours  
**Files:**
- `crates/fortify-core/src/trust.rs`
- `crates/fortify-http/src/middleware.rs`

**Vulnerable Patterns:**
```rust
// ❌ VULNERABLE - Can panic if token format invalid
let parts: Vec<&str> = token.split('.').collect();
let payload = parts[1].unwrap();

// ❌ VULNERABLE - Can panic if base64 invalid
let decoded = base64::decode(payload).unwrap();

// ❌ VULNERABLE - Can panic if JSON invalid
let session: SessionToken = serde_json::from_slice(&decoded).unwrap();
```

**Safe Replacements:**
```rust
// ✅ SAFE - Explicit error handling at each step
let parts: Vec<&str> = token.split('.').collect();
let payload = parts.get(1)
    .ok_or(TokenError::MalformedToken)?;

let decoded = base64::decode(payload)
    .map_err(|_| TokenError::InvalidBase64)?;

let session: SessionToken = serde_json::from_slice(&decoded)
    .map_err(|e| TokenError::InvalidJson(e.to_string()))?;
```

**Sub-tasks:**
- [ ] Audit all token parsing code
- [ ] Replace unwraps with `?` operator
- [ ] Add `TokenError` enum with variants:
  - `MalformedToken`
  - `InvalidBase64`
  - `InvalidJson`
  - `InvalidSignature`
- [ ] Test with malformed tokens

---

### Task 4: Fix Lock Operations
**Status:** ⬜ Not Started  
**Estimated Time:** 2 hours  
**Files:**
- `crates/fortify-http/src/admin.rs` (20+ lock unwraps)
- `crates/fortify-core/src/logging.rs`

**The Problem:**
```rust
// ❌ VULNERABLE - If ANY thread panics holding this lock, 
// the lock becomes "poisoned" and ALL future unwraps panic
let mut inner = self.inner.write().unwrap();
```

**Safe Replacements:**
```rust
// ✅ SAFE - Recovers from poisoned lock
let mut inner = self.inner.write()
    .unwrap_or_else(|poisoned| {
        tracing::error!("Admin state lock poisoned, recovering");
        poisoned.into_inner()
    });
```

**Alternative - Propagate Error:**
```rust
// ✅ SAFE - Returns error instead of panicking
let inner = self.inner.read()
    .map_err(|_| AdminError::StatePoisoned)?;
```

**Sub-tasks:**
- [ ] Find all `.lock().unwrap()` calls
- [ ] Find all `.read().unwrap()` and `.write().unwrap()` calls
- [ ] Choose strategy per location:
  - **Recovery:** For critical paths (session validation)
  - **Propagate Error:** For admin operations (can fail gracefully)
- [ ] Add `AdminError::StatePoisoned` error variant

---

### Task 5: Add Clippy Lints
**Status:** ⬜ Not Started  
**Estimated Time:** 1 hour

**Files to Modify (add to top of lib.rs):**
- `crates/fortify-http/src/lib.rs`
- `crates/fortify-gate/src/lib.rs`
- `crates/fortify-orchestrator/src/lib.rs`
- `crates/fortify-node/src/lib.rs`
- `crates/fortify-controller/src/lib.rs`

**Add these lints:**
```rust
// Deny unwrap in production code (panics are unacceptable)
#![deny(clippy::unwrap_used)]

// Warn on expect (review case-by-case)
#![warn(clippy::expect_used)]

// Deny indexing without bounds checking
#![deny(clippy::indexing_slicing)]

// Warn on panic in production code
#![warn(clippy::panic)]
```

**Sub-tasks:**
- [ ] Add lints to all production crates
- [ ] Fix compilation errors (the goal!)
- [ ] For test code, add `#[allow(clippy::unwrap_used)]`
- [ ] Verify clean build: `cargo clippy --all -- -D warnings`

---

### Task 6: Create Fuzzing Infrastructure
**Status:** ⬜ Not Started  
**Estimated Time:** 2 hours

**Installation:**
```bash
cargo install cargo-fuzz
cd crates/fortify-http
cargo fuzz init
```

**Create Fuzz Target (HTTP headers):**
```rust
// fuzz/fuzz_targets/http_headers.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Should never panic, only return errors
        let _ = parse_headers(s);
    }
});
```

**Create Fuzz Target (Token parsing):**
```rust
// fuzz/fuzz_targets/token_parsing.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let secret = b"test_secret";
        let _ = SessionToken::from_string(s, secret);
    }
});
```

**Run Fuzzing:**
```bash
cargo fuzz run http_headers -- -max_total_time=300  # 5 minutes
cargo fuzz run token_parsing -- -max_total_time=300
```

**Sub-tasks:**
- [ ] Install cargo-fuzz
- [ ] Create fuzz directory structure
- [ ] Write HTTP header fuzz target
- [ ] Write token parsing fuzz target
- [ ] Run for 1 hour minimum each
- [ ] Fix any panics discovered

---

### Task 7: Create Malformed Input Test Suite
**Status:** ⬜ Not Started  
**Estimated Time:** 1 hour  
**File:** `tests/malformed_input_test.rs`

**Test Cases:**
```rust
#[tokio::test]
async fn test_missing_headers() {
    // Send request with no headers → 400, not panic
}

#[tokio::test]
async fn test_invalid_utf8_headers() {
    // Send invalid UTF-8 in headers → 400, not panic
}

#[tokio::test]
async fn test_malformed_cookie() {
    // Cookie with no '=' sign → 400, not panic
}

#[tokio::test]
async fn test_invalid_token_base64() {
    // Token that's not valid base64 → 401, not panic
}

#[tokio::test]
async fn test_truncated_token() {
    // Token with missing parts → 401, not panic
}

#[tokio::test]
async fn test_oversized_headers() {
    // 1000+ headers → 413 or 400, not panic
}

#[tokio::test]
async fn test_null_bytes_in_path() {
    // \0 in path → 400, not panic
}
```

**Sub-tasks:**
- [ ] Write comprehensive malformed input tests
- [ ] Run tests: `cargo test malformed_input`
- [ ] Verify no panics, proper error codes returned
- [ ] Add to CI/CD pipeline

---

### Task 8: Lock Poisoning Recovery Test
**Status:** ⬜ Not Started  
**Estimated Time:** 30 min  
**File:** `tests/lock_poison_recovery_test.rs`

**Test Case:**
```rust
#[tokio::test]
async fn test_admin_state_lock_poisoning() {
    let state = Arc::new(RwLock::new(AdminState::new()));
    
    // Thread 1: Panic while holding write lock
    let state_clone = state.clone();
    let handle = tokio::spawn(async move {
        let _guard = state_clone.write().unwrap();
        panic!("Simulated panic");
    });
    
    // Wait for panic
    let _ = handle.await;
    
    // Thread 2: Should recover from poisoned lock
    let guard = state.read()
        .unwrap_or_else(|p| p.into_inner());
    
    // Should succeed despite poisoned lock
    assert!(guard.is_some_valid_state());
}
```

---

## Completion Checklist

- [ ] Audit complete (all unwrap/expect/panic locations documented)
- [ ] HTTP header parsing - zero unwraps
- [ ] Token deserialization - zero unwraps
- [ ] Lock operations - handle poisoning
- [ ] Clippy lints added and enforced
- [ ] Fuzzing infrastructure operational
- [ ] 1+ hour fuzzing completed, no panics
- [ ] Malformed input test suite passing
- [ ] Lock poisoning recovery tests passing
- [ ] CI/CD updated with new tests

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Missing some unwraps | Medium | High | Clippy lint catches new ones |
| Lock recovery breaks state | Low | Medium | Test recovery thoroughly |
| Fuzzing finds many panics | Medium | Medium | Prioritize by attack surface |

---

## References

- [Rust Error Handling Guidelines](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [cargo-fuzz Documentation](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- Previous document: `security-hardening/02-panic-audit.md` (archived)
