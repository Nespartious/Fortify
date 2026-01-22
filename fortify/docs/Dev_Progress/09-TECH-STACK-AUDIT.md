# Technology Stack Audit & Validation

**Document ID:** AUDIT-002  
**Priority:** 🟢 LOW (Technical Debt / Due Diligence)  
**Estimated Effort:** 2-3 days (research) + varies (implementation)  
**Status:** ⬜ Not Started  
**Created:** January 22, 2026

---

## Objective

Validate that Fortify uses optimal dependencies, processes, and patterns for a Tor-only hidden service protection system. Identify any dependencies or approaches that should be changed, with justification and effort estimates.

---

## Project Context

**Mission:** Defend Tor hidden services from attacks and deanonymization attempts  
**Constraints:**
- Tor-only networking (no clearnet)
- Must handle adversarial input
- Minimal attack surface
- Resource-efficient for commodity hardware
- Privacy-preserving by design

---

## Current Technology Stack

### Core Language & Toolchain
| Component | Current | Version | Status | Notes |
|-----------|---------|---------|--------|-------|
| Language | Rust | MSRV 1.88, CI 1.92 | ✅ Optimal | Memory safety, performance |
| Build System | Cargo | Latest | ✅ Optimal | Standard Rust tooling |
| Linting | Clippy | Latest | ✅ Optimal | Enabled in CI |
| Formatting | rustfmt | Latest | ✅ Optimal | Enforced |

### Async Runtime
| Component | Current | Alternative | Recommendation |
|-----------|---------|-------------|----------------|
| Runtime | Tokio 1.x | async-std, smol | ✅ **Keep Tokio** |

**Analysis:**
- Tokio is the de-facto async runtime for Rust
- Excellent performance, mature ecosystem
- Hyper (our HTTP library) is Tokio-native
- Tor control uses Tokio's TCP streams

**Verdict:** ✅ No change needed

---

### HTTP Server
| Component | Current | Version | Alternative | Recommendation |
|-----------|---------|---------|-------------|----------------|
| HTTP Server | Hyper | 1.8 | Axum, Actix-web, Warp | 🟡 **Consider Axum** |

**Analysis:**

**Hyper (Current):**
- Low-level HTTP library
- Maximum control, minimal abstractions
- Requires manual routing, middleware
- Good for our use case (custom proxy logic)

**Axum (Alternative):**
- Built on Hyper (same foundation)
- Ergonomic routing, extractors
- Tower middleware ecosystem
- Maintained by Tokio team
- Would simplify Gate/Admin server code

**Actix-web (Not Recommended):**
- Different async model (actor-based)
- Would require significant rewrite
- Overkill for our needs

**Verdict:** 🟡 Consider migrating Gate/Admin to Axum in future (non-urgent)

**Effort if changed:** 5-7 days
- Refactor Gate server to Axum routes
- Refactor Admin server to Axum routes
- HTTP proxy layer stays on Hyper (fine-grained control needed)

---

### HTTP Client
| Component | Current | Version | Alternative | Recommendation |
|-----------|---------|---------|-------------|----------------|
| HTTP Client | Reqwest | 0.12 | ureq, Hyper client | ✅ **Keep Reqwest** |

**Analysis:**
- Reqwest is async, Tokio-native
- Built on Hyper
- Easy to use, well-maintained
- Supports timeouts, retries, connection pooling

**Verdict:** ✅ No change needed

---

### Serialization
| Component | Current | Version | Alternative | Recommendation |
|-----------|---------|---------|-------------|----------------|
| JSON | serde_json | Latest | simd-json | ✅ **Keep serde_json** |
| TOML | toml | Latest | - | ✅ Keep |
| General | serde | 1.x | - | ✅ Keep |

**Analysis:**
- Serde is the Rust standard for serialization
- simd-json is faster but adds complexity
- Our JSON payloads are small (not a bottleneck)

**Verdict:** ✅ No change needed

---

### TUI Framework
| Component | Current | Version | Alternative | Recommendation |
|-----------|---------|---------|-------------|----------------|
| TUI | Ratatui | 0.29 | Crossterm only, Cursive | ✅ **Keep Ratatui** |

**Analysis:**
- Ratatui is actively maintained (fork of tui-rs)
- Good performance, flexible
- Large community, many examples

**Verdict:** ✅ No change needed

---

### Cryptography
| Component | Current | Version | Alternative | Recommendation |
|-----------|---------|---------|-------------|----------------|
| Hashing | SHA-256 (std) | - | ring, RustCrypto | 🔍 **Audit Needed** |
| Random | rand | Latest | getrandom | ✅ Keep |
| Base64 | base64 | Latest | - | ✅ Keep |

**Analysis:**
- Need to audit what crypto primitives are used
- Should use audited crates (ring or RustCrypto) for any security-critical operations
- Token signing/verification should use HMAC or similar

**Action Items:**
- [ ] Audit crypto usage across codebase
- [ ] Ensure security-critical ops use audited crates
- [ ] Document crypto choices

**Effort if changed:** 1-2 days (crypto audit and possible migration)

---

### Error Handling
| Component | Current | Version | Alternative | Recommendation |
|-----------|---------|---------|-------------|----------------|
| Errors | thiserror | 2.0 | anyhow, eyre | ✅ **Keep thiserror** |

**Analysis:**
- thiserror for library code (typed errors)
- anyhow for application code (simpler error chains)
- Current usage is appropriate

**Verdict:** ✅ No change needed

---

### Logging & Tracing
| Component | Current | Version | Alternative | Recommendation |
|-----------|---------|---------|-------------|----------------|
| Logging | tracing | Latest | log, slog | ✅ **Keep tracing** |
| Subscriber | tracing-subscriber | Latest | - | ✅ Keep |

**Analysis:**
- tracing is the modern standard
- Async-aware, span-based
- Good integration with Tokio

**Verdict:** ✅ No change needed

---

### Tor Integration
| Component | Current | Approach | Alternative | Recommendation |
|-----------|---------|----------|-------------|----------------|
| Tor Control | Raw TCP | Control port | Arti (Tor in Rust) | 🟡 **Monitor Arti** |

**Analysis:**

**Current Approach:**
- Connect to Tor control port via TCP
- Send raw control protocol commands
- Works with existing Tor daemon

**Arti (Future):**
- Pure Rust Tor implementation
- Embeddable (no external Tor daemon)
- Still maturing, not production-ready for all use cases
- Would simplify deployment

**Verdict:** 🟡 Keep current approach, monitor Arti for v2.0+

**Effort if changed:** 10-15 days (significant refactor)

---

### Process Considerations

#### 1. Dependency Pinning
| Aspect | Current | Recommendation |
|--------|---------|----------------|
| Cargo.lock | Committed | ✅ Keep committed |
| Version specs | Some wildcards | 🟡 Pin major versions |

**Action:** Review Cargo.toml for overly broad version ranges

#### 2. Security Scanning
| Aspect | Current | Recommendation |
|--------|---------|----------------|
| cargo-audit | ✅ In CI | ✅ Keep |
| cargo-deny | ❌ Not used | 🟡 Consider adding |

**cargo-deny benefits:**
- License checking
- Duplicate detection
- Advisory database checks
- Ban specific crates

**Effort to add:** 0.5 days

#### 3. Fuzzing
| Aspect | Current | Recommendation |
|--------|---------|----------------|
| cargo-fuzz | ⬜ Planned | 🔴 Implement for parsers |

**Priority targets:**
- HTTP header parsing
- Token/session parsing
- CAPTCHA input validation

**Effort:** 2-3 days (covered in Panic Audit sprint)

#### 4. MSRV Policy
| Aspect | Current | Recommendation |
|--------|---------|----------------|
| MSRV | 1.88 | ✅ Appropriate |
| CI Version | 1.92 | ✅ Appropriate |

**Analysis:** 
- Conservative MSRV allows broader deployment
- Latest in CI catches new warnings

---

## Recommendations Summary

### ✅ Keep (No Changes Needed)
- Tokio async runtime
- Reqwest HTTP client
- Serde/serde_json serialization
- Ratatui TUI framework
- thiserror for errors
- tracing for logging
- Tor control port approach

### 🟡 Consider (Non-Urgent Improvements)
| Change | Benefit | Effort | Priority |
|--------|---------|--------|----------|
| Add cargo-deny | License/security checks | 0.5 days | Medium |
| Migrate Gate to Axum | Cleaner routing code | 5-7 days | Low |
| Pin dep versions | Reproducible builds | 0.5 days | Medium |

### 🔴 Action Required (Before Beta)
| Change | Benefit | Effort | Priority |
|--------|---------|--------|----------|
| Crypto audit | Verify secure primitives | 1-2 days | High |
| Fuzz targets | Find parser bugs | 2-3 days | High |

### 🔮 Future (v2.0+)
| Change | Benefit | Effort | Priority |
|--------|---------|--------|----------|
| Arti integration | Embedded Tor | 10-15 days | Future |

---

## Additional Inspections Before New Dev Work

### 1. Security Hardening Review
- [ ] Review all network-facing code paths
- [ ] Audit input validation on all parsers
- [ ] Check for path traversal in file operations
- [ ] Verify no sensitive data in logs

### 2. Performance Baseline
- [ ] Benchmark request handling latency
- [ ] Measure memory usage under load
- [ ] Profile CAPTCHA generation time
- [ ] Identify any O(n²) algorithms

### 3. Documentation Completeness
- [ ] API documentation coverage
- [ ] Operator deployment guide
- [ ] Configuration reference
- [ ] Threat model documentation

### 4. Test Coverage Analysis
- [ ] Measure current test coverage
- [ ] Identify critical paths without tests
- [ ] Add tests for edge cases
- [ ] Document testing strategy

### 5. Error Message Audit
- [ ] Ensure errors don't leak sensitive info
- [ ] Verify user-facing messages are helpful
- [ ] Check error codes are consistent

---

## Implementation Tasks

### Task 1: Crypto Audit
**Priority:** 🔴 High  
**Effort:** 1-2 days

```bash
# Find crypto usage
grep -rn "sha\|hmac\|hash\|encrypt\|decrypt\|sign\|verify" crates/ --include="*.rs"
grep -rn "ring\|rustcrypto\|openssl\|sodiumoxide" crates/ --include="*.rs"
```

### Task 2: Add cargo-deny
**Priority:** 🟡 Medium  
**Effort:** 0.5 days

```bash
cargo install cargo-deny
cargo deny init
# Configure deny.toml
# Add to CI workflow
```

### Task 3: Dependency Version Audit
**Priority:** 🟡 Medium  
**Effort:** 0.5 days

- Review Cargo.toml for `*` or `^0.x` versions
- Pin to specific major.minor where appropriate
- Ensure Cargo.lock is committed

### Task 4: Security Path Review
**Priority:** 🔴 High  
**Effort:** 2-3 days

- Trace all network input handling
- Verify sanitization at boundaries
- Document trust boundaries

---

## References

- [Rust Secure Coding Guidelines](https://anssi-fr.github.io/rust-guide/)
- [Tokio Best Practices](https://tokio.rs/tokio/topics)
- [Hyper Security Considerations](https://hyper.rs/)
- [Arti Project](https://gitlab.torproject.org/tpo/core/arti)
