# Critical Issue #2: Comprehensive Panic Audit

**Priority:** 🔴 CRITICAL (Beta Blocker)  
**Estimated Effort:** 3-5 days  
**Status:** Not Started

---

## Overview

**Problem:** `unwrap()` and `expect()` calls on attacker-controlled input can cause panics, resulting in instant DoS. In async contexts, panics can poison the task executor. Lock poisoning can cascade failures across all threads.

**Goal:** Systematically audit and replace all unsafe panic paths in network-facing code with proper error handling.

**Success Criteria:**
- [ ] Zero unwraps on untrusted input in production code
- [ ] Lock poisoning handled gracefully (no cascading failures)
- [ ] Fuzzing infrastructure to discover panic paths
- [ ] Clippy lints enforced: `#![deny(clippy::unwrap_used)]`
- [ ] Service remains running under malformed/malicious input

---

## Panic Risk Categories

### 🔴 CRITICAL - Must Fix (Attacker-Controlled Input)
Network-facing code that processes untrusted data:
- HTTP header parsing
- Cookie/token deserialization
- Request body parsing
- WebSocket message handling
- Tor control protocol responses

### 🟡 HIGH - Should Fix (Internal Operations)
Shared state access that can poison locks:
- Mutex/RwLock operations
- Shared session storage
- Admin panel state

### 🟢 LOW - Acceptable (Validated at Startup)
Code that runs once at initialization:
- Configuration file parsing
- Environment variable reading
- Startup validation checks

### ⚪ SAFE - No Action Needed
Test code and documentation:
- Unit tests
- Integration tests
- Example code

---

## Implementation Steps

### Phase 1: Comprehensive Panic Audit

**Status:** ⬜ Not Started

**Task 1.1:** Find all `unwrap()` calls in production code

**Command:**
```bash
cd /home/shadowbox/Fortify/Fortify/fortify

# Find all unwrap() calls (exclude tests and build artifacts)
grep -rn "\.unwrap()" crates/fortify-*/src/ \
  | grep -v "/test" \
  | grep -v "_test.rs" \
  | grep -v "tests/" \
  > /tmp/unwrap_audit.txt

echo "Total unwrap() calls found:"
wc -l /tmp/unwrap_audit.txt
```

**Expected Output:** List of all unwrap locations (estimated 50-100 instances)

**Deliverable:**
- [ ] Create `/tmp/unwrap_audit.txt`
- [ ] Categorize each unwrap by risk level (CRITICAL/HIGH/LOW/SAFE)
- [ ] Create audit spreadsheet with columns:
  - File:Line
  - Code Context
  - Risk Level (CRITICAL/HIGH/LOW/SAFE)
  - Input Source (network/user/config/internal)
  - Fix Priority (P0/P1/P2/P3)
  - Status (Not Started/In Progress/Complete)

---

**Task 1.2:** Find all `expect()` calls in production code

**Command:**
```bash
grep -rn "\.expect(" crates/fortify-*/src/ \
  | grep -v "/test" \
  | grep -v "_test.rs" \
  | grep -v "tests/" \
  > /tmp/expect_audit.txt

echo "Total expect() calls found:"
wc -l /tmp/expect_audit.txt
```

**Deliverable:**
- [ ] Create `/tmp/expect_audit.txt`
- [ ] Add to same audit spreadsheet
- [ ] Mark expect() on untrusted input as CRITICAL

---

**Task 1.3:** Find all explicit `panic!()` calls

**Command:**
```bash
grep -rn "panic!" crates/fortify-*/src/ \
  | grep -v "/test" \
  | grep -v "_test.rs" \
  | grep -v "tests/" \
  > /tmp/panic_audit.txt

echo "Total panic!() calls found:"
wc -l /tmp/panic_audit.txt
```

**Deliverable:**
- [ ] Create `/tmp/panic_audit.txt`
- [ ] Add to audit spreadsheet
- [ ] All explicit panics should be in initialization code only

---

### Phase 2: Fix Critical Path Panics (Network Input)

**Status:** ⬜ Not Started

**Task 2.1:** HTTP Header Parsing

**Files to Audit:**
- `crates/fortify-http/src/middleware.rs`
- `crates/fortify-http/src/routing.rs`
- `crates/fortify-http/src/proxy.rs`
- `crates/fortify-gate/src/server.rs`

**Common Vulnerable Patterns:**
```rust
// ❌ VULNERABLE - Can panic if header missing
let cookie = headers.get("cookie").unwrap();

// ❌ VULNERABLE - Can panic if header invalid UTF-8
let session = cookie.to_str().unwrap();

// ❌ VULNERABLE - Can panic if split fails
let parts: Vec<&str> = header.split('=').collect();
let value = parts[1]; // Index panic if no '=' found
```

**Safe Patterns:**
```rust
// ✅ SAFE - Returns error if header missing
let cookie = headers
    .get("cookie")
    .ok_or(HttpError::MissingCookie)?;

// ✅ SAFE - Returns error if invalid UTF-8
let session = cookie
    .to_str()
    .map_err(|_| HttpError::InvalidCookie)?;

// ✅ SAFE - Pattern matching, no index access
let parts: Vec<&str> = header.split('=').collect();
let value = match parts.as_slice() {
    [_, val] => val,
    _ => return Err(HttpError::MalformedHeader),
};

// ✅ EVEN BETTER - Use split_once()
let (key, value) = header
    .split_once('=')
    .ok_or(HttpError::MalformedHeader)?;
```

**Steps:**
1. [ ] Search for `.get().unwrap()` pattern on headers
2. [ ] Search for `.to_str().unwrap()` on header values
3. [ ] Search for array indexing on split results
4. [ ] Replace all with safe error handling
5. [ ] Add custom error types: `HttpError::MissingHeader`, `HttpError::InvalidHeader`

**Test Commands:**
```bash
# Test with missing headers
curl -X POST http://localhost:8082 -H "Content-Length: 0"

# Test with invalid UTF-8 in header
curl -X POST http://localhost:8082 -H "X-Session: $(printf '\xff\xfe')"

# Test with malformed cookie
curl http://localhost:8082 -H "Cookie: no_equals_sign"
```

**Expected Result:** Service returns 400 Bad Request, does not panic

---

**Task 2.2:** Token/Session Deserialization

**Files to Audit:**
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

**Safe Patterns:**
```rust
// ✅ SAFE - Explicit error handling at each step
let parts: Vec<&str> = token.split('.').collect();
let payload = parts.get(1)
    .ok_or(TokenError::MalformedToken)?;

let decoded = base64::decode(payload)
    .map_err(|_| TokenError::InvalidBase64)?;

let session: SessionToken = serde_json::from_slice(&decoded)
    .map_err(|e| TokenError::InvalidJson(e.to_string()))?;

// Verify HMAC signature
session.verify(&secret_key)?;
```

**Steps:**
1. [ ] Audit all token parsing code
2. [ ] Replace unwraps with `?` operator
3. [ ] Add custom error types: `TokenError::MalformedToken`, `TokenError::InvalidSignature`
4. [ ] Test with malformed tokens

**Test Commands:**
```bash
# Test with invalid base64
curl http://localhost:8082 -H "Cookie: session=not_valid_base64"

# Test with truncated token
curl http://localhost:8082 -H "Cookie: session=abc"

# Test with invalid JSON
curl http://localhost:8082 -H "Cookie: session=$(echo 'not json' | base64)"
```

**Expected Result:** Service returns 401 Unauthorized or 400 Bad Request, does not panic

---

**Task 2.3:** Tor Control Protocol Response Parsing

**Files to Audit:**
- `crates/fortify-orchestrator/src/tor.rs`

**Location:** 
- `extract_service_id()` - Line ~560
- `extract_private_key()` - Line ~570
- `parse_response()` - Throughout

**Vulnerable Patterns:**
```rust
// ❌ VULNERABLE - Tor response might not contain ServiceID
let service_id = response.lines()
    .find(|line| line.starts_with("ServiceID="))
    .unwrap()
    .strip_prefix("ServiceID=")
    .unwrap();
```

**Safe Patterns:**
```rust
// ✅ SAFE - Returns error if ServiceID missing
let service_id = response.lines()
    .find(|line| line.starts_with("ServiceID="))
    .and_then(|line| line.strip_prefix("ServiceID="))
    .ok_or_else(|| OrchestratorError::TorConfigError(
        "Tor ADD_ONION response missing ServiceID".to_string()
    ))?;
```

**Current Code Review (from earlier read):**
```rust
// Line 568 - GOOD, already has error handling
.ok_or_else(|| {
    OrchestratorError::TorConfigError(
        "Tor ADD_ONION response missing ServiceID".into(),
    )
})?
```

**Steps:**
1. [ ] Verify `extract_service_id()` has no unwraps (ALREADY FIXED ✅)
2. [ ] Verify `extract_private_key()` has no unwraps (ALREADY FIXED ✅)
3. [ ] Audit any other Tor response parsing
4. [ ] Test with malformed Tor responses (mock Tor daemon)

---

**Task 2.4:** WebSocket Message Parsing

**Files to Audit:**
- `crates/fortify-http/src/admin.rs` (if WebSocket implemented)
- Admin panel message handlers

**Vulnerable Patterns:**
```rust
// ❌ VULNERABLE - Message might not be valid JSON
let msg: AdminCommand = serde_json::from_str(&text).unwrap();
```

**Safe Patterns:**
```rust
// ✅ SAFE - Returns error to client
let msg: AdminCommand = match serde_json::from_str(&text) {
    Ok(cmd) => cmd,
    Err(e) => {
        log::warn!("Invalid admin command JSON: {}", e);
        // Send error response to client
        ws_sender.send(Message::Text(
            json!({"error": "Invalid JSON"}).to_string()
        )).await?;
        continue;
    }
};
```

**Steps:**
1. [ ] Find WebSocket message handlers
2. [ ] Replace JSON parsing unwraps with error responses
3. [ ] Test with invalid WebSocket messages

---

### Phase 3: Fix High-Priority Lock Operations

**Status:** ⬜ Not Started

**Task 3.1:** Audit Lock Operations in Admin State

**File:** `crates/fortify-http/src/admin.rs`

**Current Code (from earlier grep):**
```rust
// Line 205 and 20+ other instances
let mut inner = self.inner.write().unwrap();
let inner = self.inner.read().unwrap();
let mut guard = self.state.inner.lock().unwrap();
```

**The Problem:**
- If ANY thread panics while holding this lock, the lock becomes "poisoned"
- All future `.unwrap()` calls on that lock will also panic
- This causes **cascading failure** across all requests

**Safe Pattern - Recover from Poisoned Locks:**
```rust
// ✅ SAFE - Recovers from poisoned lock
let mut inner = self.inner.write()
    .unwrap_or_else(|poisoned| {
        log::error!("Admin state lock poisoned, recovering");
        // Get the inner data despite poison
        poisoned.into_inner()
    });
```

**Alternative - Propagate Error:**
```rust
// ✅ SAFE - Returns error instead of panicking
let inner = self.inner.read()
    .map_err(|_| AdminError::StatePoisoned)?;
```

**Steps:**
1. [ ] Find all `.lock().unwrap()` calls
2. [ ] Find all `.read().unwrap()` and `.write().unwrap()` calls
3. [ ] Choose strategy per location:
   - **Recovery:** For critical paths (session validation, request handling)
   - **Propagate Error:** For admin panel operations (can fail gracefully)
4. [ ] Add `AdminError::StatePoisoned` error variant
5. [ ] Test by inducing panic in lock-holding code

**Files to Modify:**
- [ ] `crates/fortify-http/src/admin.rs` (20+ lock unwraps)
- [ ] `crates/fortify-core/src/logging.rs` (mutex unwraps)
- [ ] Any other shared state access

---

**Task 3.2:** Add Lock Poisoning Tests

**File:** `tests/lock_poison_test.rs`

**Test Case:**
```rust
#[tokio::test]
async fn test_lock_poisoning_recovery() {
    let state = Arc::new(RwLock::new(AdminState::new()));
    
    // Thread 1: Panic while holding write lock
    let state_clone = state.clone();
    let handle = tokio::spawn(async move {
        let mut guard = state_clone.write().unwrap();
        guard.update_something();
        panic!("Simulated panic");
    });
    
    // Wait for panic
    let _ = handle.await;
    
    // Thread 2: Should recover from poisoned lock
    let state_clone = state.clone();
    let result = tokio::spawn(async move {
        let guard = state_clone.read()
            .unwrap_or_else(|p| p.into_inner());
        guard.get_something()
    }).await;
    
    // Should succeed despite poisoned lock
    assert!(result.is_ok());
}
```

**Steps:**
- [ ] Write tests for lock poisoning scenarios
- [ ] Verify service recovers gracefully
- [ ] Add to integration test suite

---

### Phase 4: Add Clippy Lints

**Status:** ⬜ Not Started

**Task 4.1:** Add lints to production crates

**Files to Modify:**
- `crates/fortify-http/src/lib.rs`
- `crates/fortify-gate/src/lib.rs`
- `crates/fortify-orchestrator/src/lib.rs`
- `crates/fortify-node/src/lib.rs`
- `crates/fortify-controller/src/lib.rs`

**Add to top of each lib.rs:**
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

**Note:** These lints will cause compilation errors on existing unwraps. That's the goal - forces us to fix them.

**Steps:**
1. [ ] Add lints to all production crates
2. [ ] Fix compilation errors (replace unwraps)
3. [ ] For test code, add `#[allow(clippy::unwrap_used)]` at module level
4. [ ] Verify clean build: `cargo clippy --all -- -D warnings`

---

**Task 4.2:** Configure Cargo.toml for strict lints

**File:** `Cargo.toml` (workspace root)

**Add to `[workspace]` section:**
```toml
[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "warn"
indexing_slicing = "deny"
panic = "warn"
```

**Steps:**
- [ ] Add workspace-level lints
- [ ] Verify inherited by all crates
- [ ] Run `cargo clippy --workspace` to verify

---

### Phase 5: Fuzzing Infrastructure

**Status:** ⬜ Not Started

**Task 5.1:** Set up cargo-fuzz

**Installation:**
```bash
cargo install cargo-fuzz
```

**Create fuzzing targets:**
```bash
cd crates/fortify-http
cargo fuzz init
```

**Deliverable:**
- [ ] Install cargo-fuzz
- [ ] Create fuzz/ directory structure
- [ ] Write fuzz targets for:
  - HTTP header parsing
  - Token deserialization
  - Cookie parsing
  - WebSocket message parsing

---

**Task 5.2:** Create HTTP Header Fuzz Target

**File:** `crates/fortify-http/fuzz/fuzz_targets/http_headers.rs`

**Fuzz Target:**
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use http::HeaderMap;

fuzz_target!(|data: &[u8]| {
    // Try to parse arbitrary data as HTTP headers
    if let Ok(s) = std::str::from_utf8(data) {
        // Should never panic, only return errors
        let _ = parse_headers(s);
    }
});

fn parse_headers(input: &str) -> Result<HeaderMap, Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();
    for line in input.lines() {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(
                key.trim().parse()?,
                value.trim().parse()?
            );
        }
    }
    Ok(headers)
}
```

**Run Fuzz Test:**
```bash
cd crates/fortify-http
cargo fuzz run http_headers -- -max_total_time=300  # 5 minutes
```

**Expected Result:** No panics discovered, only Result errors

**Steps:**
- [ ] Write fuzz target for HTTP headers
- [ ] Run for 1 hour minimum
- [ ] Fix any panics discovered
- [ ] Add to CI/CD for continuous fuzzing

---

**Task 5.3:** Create Token Parsing Fuzz Target

**File:** `crates/fortify-core/fuzz/fuzz_targets/token_parsing.rs`

**Fuzz Target:**
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use fortify_core::trust::SessionToken;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Try to parse arbitrary data as session token
        // Should never panic, only return errors
        let secret = b"test_secret_key_for_fuzzing";
        let _ = SessionToken::from_string(s, secret);
    }
});
```

**Steps:**
- [ ] Write fuzz target for token parsing
- [ ] Run for 1 hour minimum
- [ ] Fix any panics discovered

---

### Phase 6: Integration Tests

**Status:** ⬜ Not Started

**Task 6.1:** Malformed Input Test Suite

**File:** `tests/malformed_input_test.rs`

**Test Cases:**
```rust
#[tokio::test]
async fn test_missing_headers() {
    // Send request with no headers
    // Should return 400, not panic
}

#[tokio::test]
async fn test_invalid_utf8_headers() {
    // Send request with invalid UTF-8 in headers
    // Should return 400, not panic
}

#[tokio::test]
async fn test_malformed_cookie() {
    // Send cookie with no '=' sign
    // Should return 400, not panic
}

#[tokio::test]
async fn test_invalid_token_base64() {
    // Send token that's not valid base64
    // Should return 401, not panic
}

#[tokio::test]
async fn test_truncated_token() {
    // Send token with missing parts
    // Should return 401, not panic
}

#[tokio::test]
async fn test_oversized_headers() {
    // Send request with 1000+ headers
    // Should return 413 or 400, not panic
}

#[tokio::test]
async fn test_null_bytes_in_path() {
    // Send request with \0 in path
    // Should return 400, not panic
}

#[tokio::test]
async fn test_extremely_long_header_value() {
    // Send 10MB header value
    // Should return 413, not panic
}
```

**Steps:**
- [ ] Write comprehensive malformed input tests
- [ ] Run tests: `cargo test malformed_input_test`
- [ ] Verify no panics, proper error codes returned
- [ ] Add to CI/CD pipeline

---

**Task 6.2:** Lock Poisoning Recovery Test

**File:** `tests/lock_poison_recovery_test.rs`

**Test Case:**
```rust
#[tokio::test]
async fn test_admin_state_lock_poisoning() {
    // Simulate panic in lock-holding code
    // Verify subsequent requests recover gracefully
    // Service should NOT cascade failures
}
```

**Steps:**
- [ ] Write lock poisoning tests
- [ ] Verify recovery mechanism works
- [ ] Add to integration test suite

---

### Phase 7: Documentation

**Status:** ⬜ Not Started

**Task 7.1:** Update CONTRIBUTING.md with panic guidelines

**File:** `CONTRIBUTING.md`

**Add Section:**
```markdown
## Error Handling Guidelines

### Never Use unwrap() on Network Input

❌ **NEVER:**
```rust
let cookie = headers.get("cookie").unwrap();
```

✅ **ALWAYS:**
```rust
let cookie = headers.get("cookie")
    .ok_or(HttpError::MissingCookie)?;
```

### Lock Poisoning Recovery

All shared state access must handle lock poisoning:

```rust
let guard = self.state.lock()
    .unwrap_or_else(|poisoned| {
        log::error!("Lock poisoned, recovering");
        poisoned.into_inner()
    });
```

### Clippy Lints

Our codebase denies:
- `clippy::unwrap_used`
- `clippy::indexing_slicing`

These will cause compilation errors. Use proper error handling instead.
```

**Steps:**
- [ ] Add error handling guidelines to CONTRIBUTING.md
- [ ] Document safe patterns vs vulnerable patterns
- [ ] Add examples for common scenarios

---

**Task 7.2:** Update prof_review.md status

**File:** `docs/research/prof_review.md`

**Change:** Mark "Incomplete Error Handling (unwrap() and expect())" section as `valid-addressed` after implementation

---

## Completion Checklist

**Phase 1: Audit**
- [ ] All unwrap() calls found and categorized
- [ ] All expect() calls found and categorized
- [ ] All panic!() calls found and categorized
- [ ] Audit spreadsheet created

**Phase 2: Critical Path Fixes**
- [ ] HTTP header parsing - no unwraps
- [ ] Token deserialization - no unwraps
- [ ] Tor control responses - verified safe
- [ ] WebSocket parsing - no unwraps

**Phase 3: Lock Operations**
- [ ] Admin state locks handle poisoning
- [ ] Logging locks handle poisoning
- [ ] Lock poisoning tests written

**Phase 4: Clippy Lints**
- [ ] Lints added to all production crates
- [ ] Clean compilation with lints enforced
- [ ] Workspace lints configured

**Phase 5: Fuzzing**
- [ ] cargo-fuzz installed
- [ ] HTTP header fuzz target written
- [ ] Token parsing fuzz target written
- [ ] 1+ hour fuzzing completed, no panics

**Phase 6: Integration Tests**
- [ ] Malformed input test suite written
- [ ] Lock poisoning recovery tests written
- [ ] All tests passing
- [ ] Added to CI/CD

**Phase 7: Documentation**
- [ ] CONTRIBUTING.md updated
- [ ] Error handling guidelines documented
- [ ] prof_review.md updated

**Final Validation:**
- [ ] `cargo clippy --all -- -D warnings` passes
- [ ] Fuzzing finds no panics after 4+ hours
- [ ] Malformed input tests all pass
- [ ] Service handles poisoned locks gracefully
- [ ] Ready for Beta release

---

## Known Unwrap Locations (Preliminary)

**From Initial Grep (20+ matches found):**

1. `fortify-core/src/logging.rs:48` - `self.state.inner.lock().unwrap()`
2. `fortify-core/src/logging.rs:59` - `self.state.inner.lock().unwrap()`
3. `fortify-http/src/admin.rs:205` - `self.inner.write().unwrap()`
4. `fortify-http/src/admin.rs:210` - `self.inner.read().unwrap()`
5. `fortify-http/src/admin.rs:215` - `self.inner.read().unwrap()`
6. ... (20+ more in admin.rs)

**Priority:**
- Admin.rs locks: HIGH (can poison on panic)
- Logging.rs locks: HIGH (used throughout)
- More to be discovered in Phase 1 audit

---

## Risk Assessment

**Implementation Risks:** 🟡 MEDIUM
- Requires careful refactoring of many call sites
- Must test error paths thoroughly
- Lock poisoning recovery needs validation

**Breaking Change Risk:** 🟢 LOW
- Most changes are internal error handling
- API surfaces mostly unchanged
- May expose more error variants (good!)

**Security Impact:** 🟢 HIGH POSITIVE
- Prevents panic-induced DoS attacks
- Prevents cascading failures from lock poisoning
- More robust error handling overall

**Operational Impact:** 🟢 POSITIVE
- Better error messages for debugging
- Service stays running under attack
- Easier to diagnose issues from logs

---

## Questions to Answer

1. **Q:** Should we allow any unwraps in production code?
   **A:** Only in initialization/startup code where fail-fast is appropriate. Network-facing code: zero unwraps.

2. **Q:** What about `expect()` - is it better than `unwrap()`?
   **A:** Slightly better (provides context), but still panics. Use `?` operator with proper Result types instead.

3. **Q:** How do we handle lock poisoning - recover or propagate error?
   **A:** Depends on criticality:
   - Critical paths (session validation): Recover
   - Admin operations: Propagate error

4. **Q:** Should we run fuzzing in CI/CD?
   **A:** Yes, but time-limited (5 minutes per target). Longer fuzzing runs nightly or weekly.

5. **Q:** What if we find panics in dependencies (tokio, hyper, etc.)?
   **A:** Report to upstream. Our code should handle any errors from dependencies gracefully.

---

**Status Legend:**
- ⬜ Not Started
- 🟦 In Progress  
- ✅ Complete
- ⚠️ Blocked
