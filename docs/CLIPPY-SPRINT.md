# 🏃 Clippy Lint Sprint Guide

> **Purpose**: Systematic guide to fix all ~300 clippy lints in priority order
> **Estimated Total Time**: 3-4 hours
> **Created**: 2026-01-21

## Sprint Overview

| Phase | Category | Count | Priority | Est. Time |
|-------|----------|-------|----------|-----------|
| 1 | Safety-Critical Casts | ~60 | 🔴 HIGH | 60 min |
| 2 | Async/Ownership Issues | ~33 | 🟠 MEDIUM | 30 min |
| 3 | Format String Cleanup | ~110 | 🟡 LOW | 45 min |
| 4 | Code Organization | ~50 | 🟢 STYLE | 30 min |
| 5 | Remaining Pedantic | ~50 | 🟢 STYLE | 30 min |

---

## Phase 1: Safety-Critical Casting Issues (🔴 HIGH PRIORITY)

These lints catch potential runtime panics, data corruption, and undefined behavior.

### 1.1 `cast_possible_truncation` (~10 occurrences)
**Risk**: Data loss when casting larger integers to smaller ones on 64-bit systems.

```rust
// BAD: Can truncate on 64-bit systems
let x: u32 = some_usize as u32;

// GOOD: Use try_into() with error handling
let x: u32 = some_usize.try_into().unwrap_or(u32::MAX);

// GOOD: Use saturating conversion
let x: u32 = some_usize.min(u32::MAX as usize) as u32;
```

**Files to check**:
- [ ] `fortify-gate/src/server.rs`
- [ ] `fortify-orchestrator/src/lib.rs`
- [ ] `fortify-http/src/lib.rs`
- [ ] `fortify-node/src/lib.rs`

### 1.2 `cast_sign_loss` (~5 occurrences)
**Risk**: Converting negative numbers to unsigned types causes wrap-around.

```rust
// BAD: Negative becomes huge positive
let x: u32 = some_i32 as u32;  // -1 becomes 4294967295!

// GOOD: Check for negative first
let x: u32 = some_i32.try_into().unwrap_or(0);

// GOOD: Use unsigned_abs() for absolute values
let x: u32 = some_i32.unsigned_abs();
```

**Files to check**:
- [ ] `fortify-orchestrator/src/lib.rs`
- [ ] `fortify-http/src/admin.rs`

### 1.3 `cast_possible_wrap` (~9 occurrences)
**Risk**: Integer overflow when casting between signed/unsigned of same size.

```rust
// BAD: Can wrap around
let x: i32 = some_u32 as i32;  // Large u32 becomes negative!

// GOOD: Saturate or check bounds
let x: i32 = some_u32.min(i32::MAX as u32) as i32;
```

**Files to check**:
- [ ] `fortify-controller/src/http.rs`
- [ ] `fortify-gate/src/server.rs`

### 1.4 `cast_precision_loss` (~15 occurrences)
**Risk**: Loss of precision when converting integers to floats.

```rust
// BAD: u64 has 64 bits, f64 mantissa only 52
let x: f64 = some_u64 as f64;  // Loses precision for large values

// ACCEPTABLE: If precision loss is acceptable, add allow
#[allow(clippy::cast_precision_loss)]
let percentage: f64 = count as f64 / total as f64;

// BETTER: Use explicit conversion noting precision
let x = f64::from(some_u32);  // u32 → f64 is lossless
```

### 1.5 `unchecked_time_subtraction` (1 occurrence)
**Risk**: Panics if subtracting Duration results in negative time.

```rust
// BAD: Panics if window > now
let window_start = now - self.window;

// GOOD: Use checked or saturating subtraction
let window_start = now.checked_sub(self.window).unwrap_or(UNIX_EPOCH);
let window_start = now.saturating_sub(self.window);
```

**File**: `fortify-http/src/lib.rs:131`

---

## Phase 2: Async & Ownership Issues (🟠 MEDIUM PRIORITY)

### 2.1 `unused_async` (~21 occurrences)
**Risk**: Unnecessary async overhead, confusing API.

```rust
// BAD: No await in function body
pub async fn get_count(&self) -> usize {
    self.items.len()  // No await!
}

// GOOD: Remove async if not needed
pub fn get_count(&self) -> usize {
    self.items.len()
}

// OR: If async is intentional for API consistency, add allow
#[allow(clippy::unused_async)]
pub async fn get_count(&self) -> usize { ... }
```

**Files with most occurrences**:
- [ ] `fortify-orchestrator/src/lib.rs` (8)
- [ ] `fortify-http/src/lib.rs` (5)
- [ ] `fortify-node/src/lib.rs` (4)
- [ ] `fortify-gate/src/server.rs` (4)

### 2.2 `needless_pass_by_value` (~12 occurrences)
**Risk**: Unnecessary cloning, performance overhead.

```rust
// BAD: Takes ownership but only reads
pub fn process(data: String) {
    println!("{}", data);
}

// GOOD: Take reference
pub fn process(data: &str) {
    println!("{}", data);
}
```

**Files to check**:
- [ ] `fortify-core/src/behavioral.rs:1194`
- [ ] `fortify-orchestrator/src/lib.rs`

### 2.3 `unused_self` (~11 occurrences)
**Risk**: Methods that could be static, misleading API.

```rust
// BAD: self not used
impl Foo {
    pub fn helper(&self, x: i32) -> i32 {
        x * 2  // self not used!
    }
}

// GOOD: Make it an associated function
impl Foo {
    pub fn helper(x: i32) -> i32 {
        x * 2
    }
}
```

---

## Phase 3: Format String Cleanup (🟡 LOW PRIORITY)

### 3.1 `uninlined_format_args` (~110 occurrences)
**Risk**: None, purely style. But easy to fix with search/replace.

```rust
// OLD STYLE
format!("Hello, {}!", name)
println!("Count: {}", count)

// NEW STYLE (Rust 2021+)
format!("Hello, {name}!")
println!("Count: {count}")
```

**Bulk fix command**:
```bash
# Use cargo clippy --fix for automatic fixes
cargo clippy --fix --allow-dirty --allow-staged -- \
  -A clippy::all -W clippy::uninlined_format_args
```

---

## Phase 4: Code Organization (🟢 STYLE)

### 4.1 `items_after_statements` (~8 occurrences)
Move function/struct definitions before the code that uses them.

### 4.2 `needless_raw_string_hashes` (~10 occurrences)
```rust
// BAD
r#"some string"#

// GOOD (if no # in string)
r"some string"
```

### 4.3 `redundant_closure_for_method_calls` (~8 occurrences)
```rust
// BAD
.map(|x| x.to_string())

// GOOD
.map(ToString::to_string)
```

### 4.4 `match_same_arms` (~7 occurrences)
Combine match arms with identical bodies.

### 4.5 `manual_let_else` (~7 occurrences)
```rust
// BAD
let x = match some_option {
    Some(v) => v,
    None => return Err(...),
};

// GOOD
let Some(x) = some_option else {
    return Err(...);
};
```

---

## Phase 5: Remaining Pedantic (🟢 STYLE)

### 5.1 `map_unwrap_or` (~11 occurrences)
```rust
// BAD
option.map(|x| x + 1).unwrap_or(0)

// GOOD
option.map_or(0, |x| x + 1)
```

### 5.2 `single_char_pattern` (~4 occurrences)
```rust
// BAD
s.split("/")

// GOOD
s.split('/')
```

### 5.3 `explicit_iter_loop` (~2 occurrences)
```rust
// BAD
for item in vec.iter() { }

// GOOD
for item in &vec { }
```

### 5.4 Miscellaneous
- `wildcard_imports` (1) - Don't use `use foo::*`
- `struct_excessive_bools` (1) - Consider using an enum
- `similar_names` (2) - Rename variables for clarity

---

## Quick Reference: Fix Commands

```bash
# Run clippy with all pedantic lints
cargo clippy --all-targets --all-features -- \
  -D warnings -D clippy::all -D clippy::pedantic \
  -A clippy::module_name_repetitions

# Auto-fix what's possible
cargo clippy --fix --allow-dirty --allow-staged

# Check specific crate
cargo clippy -p fortify-gate -- -D clippy::all -D clippy::pedantic

# Count remaining errors
cargo clippy 2>&1 | grep "^error:" | wc -l
```

---

## Progress Tracking

### Phase 1: Safety-Critical ⬜
- [ ] `cast_possible_truncation` (0/10)
- [ ] `cast_sign_loss` (0/5)
- [ ] `cast_possible_wrap` (0/9)
- [ ] `cast_precision_loss` (0/15)
- [ ] `unchecked_time_subtraction` (0/1)

### Phase 2: Async/Ownership ⬜
- [ ] `unused_async` (0/21)
- [ ] `needless_pass_by_value` (0/12)
- [ ] `unused_self` (0/11)

### Phase 3: Format Strings ⬜
- [ ] `uninlined_format_args` (0/110)

### Phase 4: Organization ⬜
- [ ] `items_after_statements` (0/8)
- [ ] `needless_raw_string_hashes` (0/10)
- [ ] `redundant_closure_for_method_calls` (0/8)
- [ ] `match_same_arms` (0/7)
- [ ] `manual_let_else` (0/7)

### Phase 5: Remaining ⬜
- [ ] All other pedantic lints (0/~50)

---

## CI Configuration After Sprint

Once all lints are fixed, update `.github/workflows/ci.yml` to remove unnecessary allows:

```yaml
# AFTER SPRINT: Strict clippy configuration
cargo clippy --all-targets --all-features -- \
  -D warnings \
  -D clippy::all \
  -D clippy::pedantic \
  -A clippy::module_name_repetitions \
  -A clippy::must_use_candidate \
  -A clippy::missing_errors_doc \
  -A clippy::missing_panics_doc
```

Only 4 allows needed (documentation-related, intentional).
