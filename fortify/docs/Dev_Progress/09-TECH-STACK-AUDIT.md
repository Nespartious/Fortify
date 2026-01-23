# Technology Stack Audit & Validation

**Document ID:** AUDIT-002  
**Priority:** 🟢 LOW (Technical Debt / Due Diligence)  
**Estimated Effort:** 2-3 days (research) + varies (implementation)  
**Status:** ✅ COMPLETE  
**Created:** January 22, 2026  
**Completed:** January 23, 2026 - PR #TBD

---

## Audit Summary

This audit was conducted on January 23, 2026 as part of Sprint 09.

### Key Findings

| Area | Status | Notes |
|------|--------|-------|
| **Cryptography** | ✅ Using RustCrypto (audited) | sha2 0.10, hmac 0.12 |
| **cargo-deny** | ✅ Already configured | In CI via security.yml |
| **Dependency Versions** | ✅ Properly pinned | No wildcards found |
| **Fuzzing** | ✅ Implemented | See PR #38 (Sprint 02 Phase 4) |

### Actions Taken

1. **Crypto Audit**: Verified all crypto uses RustCrypto crates (sha2, hmac)
2. **cargo-deny Check**: Confirmed already in `deny.toml` and CI
3. **Version Audit**: Confirmed no wildcard (`*`) version specs
4. **Fuzzing**: Implemented in separate PR #38 (Sprint 02 Phase 4)

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
| Hashing | sha2 (RustCrypto) | 0.10 | ring | ✅ **Keep RustCrypto** |
| HMAC | hmac (RustCrypto) | 0.12 | ring | ✅ **Keep RustCrypto** |
| Random | rand | Latest | getrandom | ✅ Keep |
| Base64 | base64 | Latest | - | ✅ Keep |

**Analysis (Audit Completed Jan 23, 2026):**

Crypto usage audited across codebase:
- `fortify-core/src/trust.rs`: HMAC-SHA256 for token signing/verification
- `fortify-gate/src/lib.rs`: SHA256 for hashing, HMAC for verification

All crypto uses RustCrypto crates which are:
- ✅ Actively maintained
- ✅ Widely audited by the community
- ✅ Recommended by Rust security guidelines

**Verdict:** ✅ No change needed - already using best practices

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
| Version specs | ✅ Properly pinned | ✅ No changes needed |

**Audit Result (Jan 23, 2026):** No wildcard (`*`) or overly broad version specs found in workspace Cargo.toml.

#### 2. Security Scanning
| Aspect | Current | Recommendation |
|--------|---------|----------------|
| cargo-audit | ✅ In CI | ✅ Keep |
| cargo-deny | ✅ Configured | ✅ Already implemented |

**Audit Result (Jan 23, 2026):** 
- `deny.toml` exists with proper configuration
- Running in CI via `.github/workflows/security.yml`
- Configured for: advisories, licenses, duplicates

#### 3. Fuzzing
| Aspect | Current | Recommendation |
|--------|---------|----------------|
| cargo-fuzz | ✅ Implemented | ✅ Complete |

**Audit Result (Jan 23, 2026):**
Implemented in Sprint 02 Phase 4 (PR #38):
- `fuzz_token_decode` - SessionToken parsing
- `fuzz_token_verify` - HMAC verification
- `fuzz_cookie_parse` - Cookie header parsing
- `fuzz_ip_extraction` - IP header extraction

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
- RustCrypto (sha2, hmac) for cryptography
- cargo-deny configuration

### ✅ Completed (This Audit)
| Change | Benefit | Status | PR |
|--------|---------|--------|-----|
| Crypto audit | Verify secure primitives | ✅ Complete | Sprint 09 |
| Fuzz targets | Find parser bugs | ✅ Complete | PR #38 |
| cargo-deny | License/security checks | ✅ Already existed | N/A |
| Pin dep versions | Reproducible builds | ✅ Already done | N/A |

### 🟡 Consider (Non-Urgent Improvements)
| Change | Benefit | Effort | Priority |
|--------|---------|--------|----------|
| Migrate Gate to Axum | Cleaner routing code | 5-7 days | Low |

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
**Status:** ✅ COMPLETE  
**Completed:** January 23, 2026  
**Priority:** 🔴 High  
**Effort:** 1-2 days

**Results:**
- All crypto uses RustCrypto crates (sha2 0.10, hmac 0.12)
- Token signing uses HMAC-SHA256 in `fortify-core/src/trust.rs`
- Hashing uses SHA256 in `fortify-gate/src/lib.rs`
- No insecure crypto primitives found

### Task 2: Add cargo-deny
**Status:** ✅ ALREADY EXISTS  
**Completed:** Previously implemented  
**Priority:** 🟡 Medium

**Findings:**
- `deny.toml` already configured in fortify root
- Running in CI via `.github/workflows/security.yml`
- Configured for: advisories, licenses, duplicates, bans

### Task 3: Dependency Version Audit
**Status:** ✅ COMPLETE  
**Completed:** January 23, 2026  
**Priority:** 🟡 Medium

**Results:**
- No wildcard (`*`) version specs found
- All dependencies properly pinned in workspace Cargo.toml
- Cargo.lock is committed

### Task 4: Security Path Review
**Status:** ⬜ Not Started (Future Sprint)  
**Priority:** 🔴 High  
**Effort:** 2-3 days

- Trace all network input handling
- Verify sanitization at boundaries
- Document trust boundaries

*Note: Partially addressed by fuzzing infrastructure (PR #38)*

---

## References

- [Rust Secure Coding Guidelines](https://anssi-fr.github.io/rust-guide/)
- [Tokio Best Practices](https://tokio.rs/tokio/topics)
- [Hyper Security Considerations](https://hyper.rs/)
- [Arti Project](https://gitlab.torproject.org/tpo/core/arti)
