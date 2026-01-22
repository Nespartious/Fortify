# Fortify Technical Debt & Development Priorities

> Last Updated: January 21, 2026

This document tracks known technical debt, security improvements, and development priorities for the Fortify project.

---

## 🔴 High Priority (Security-Related)

### 1. reqwest 0.11 → 0.12+ Migration (hyper 0.14 → 1.x)

**Status:** Not Started  
**Effort Estimate:** Large (2-3 days)  
**Risk Level:** Medium - Security advisory on transitive dependency

#### Problem

The project uses `reqwest = "0.11"` which depends on `rustls-pemfile` (unmaintained, RUSTSEC-2025-0134). Upgrading to reqwest 0.12+ requires migrating from hyper 0.14 to hyper 1.x, which has significant breaking changes.

#### Current Usage Analysis

**reqwest usage (4 crates):**
| Crate | Features | Usage Pattern |
|-------|----------|---------------|
| `fortify-node` | rustls-tls, socks | Async client with SOCKS proxy for .onion backends |
| `fortify-controller` | rustls-tls, socks, json | Health checks via Tor SOCKS proxy |
| `fortify-tui` | json, rustls-tls | Deployment verification |
| `fortify-http` | rustls-tls, blocking, json | Admin panel HTTP requests |

**hyper usage (5 crates, ~40 occurrences):**
| Crate | Purpose |
|-------|---------|
| `fortify-node` | HTTP server for node, request/response proxying |
| `fortify-gate` | Captcha gate server, form parsing |
| `fortify-controller` | Internal HTTP API |
| `fortify-orchestrator` | Mirror management HTTP, health proxying |
| `fortify-http` | Main HTTP server, admin panel, proxy routing |

#### Breaking Changes in hyper 1.x

From the [hyper upgrade guide](https://hyper.rs/guides/1/upgrading/):

1. **`Body` is now a trait** (was a concrete type)
   - Must use `http-body-util` for body types (`BoxBody`, `Full<Bytes>`, etc.)
   - Every `hyper::Body` reference needs updating

2. **`Server` removed** from hyper core
   - Replace with `hyper-util::server::conn::auto::Builder`
   - Need manual accept loop instead of `Server::bind()`

3. **`Client` moved** to hyper-util
   - Use `hyper_util::client::legacy::Client`
   - Mostly drop-in but import paths change

4. **`service_fn` signature changed**
   - Now in `hyper::service` (not tower)
   - Minor adjustments needed

#### Migration Plan

```
Phase 1: Preparation
  ├── Add hyper features: ["backports", "deprecated"]
  ├── Update http-body to 0.4.6+
  └── Run cargo check to see deprecation warnings

Phase 2: Dependencies
  ├── Add hyper-util = "0.1"
  ├── Add http-body-util = "0.1"  
  └── Keep hyper 0.14 temporarily for backports

Phase 3: Code Migration (per crate)
  ├── Replace Body with appropriate http-body-util type
  ├── Replace Server with hyper-util server builder
  ├── Replace Client with hyper-util legacy client
  └── Update error types (hyper::Error changes)

Phase 4: Finalize
  ├── Upgrade hyper = "1.0"
  ├── Upgrade reqwest = "0.12"
  ├── Remove ignore for RUSTSEC-2025-0134
  └── Set unmaintained = "warn" in deny.toml
```

#### Files Requiring Changes

| File | Changes Needed |
|------|----------------|
| `fortify-node/src/server.rs` | Server builder, Body type, service_fn |
| `fortify-node/src/lib.rs` | Body type, error handling |
| `fortify-gate/src/server.rs` | Server builder, Body type, form parsing |
| `fortify-controller/src/http.rs` | Server builder, Body type |
| `fortify-orchestrator/src/server.rs` | Server builder, Body type, Client |
| `fortify-http/src/lib.rs` | Server builder, Body type, Client |
| `fortify-http/src/admin.rs` | Body type, response building |
| `fortify-http/src/proxy.rs` | Body type, header handling |
| `fortify-http/src/middleware.rs` | Body type |

#### Current Workarounds

In `deny.toml`:
```toml
unmaintained = "none"  # Suppress warnings (REMOVE after migration)
ignore = ["RUSTSEC-2025-0134"]  # rustls-pemfile (REMOVE after migration)
```

In `.cargo/audit.toml`:
```toml
ignore = ["RUSTSEC-2025-0134"]  # (REMOVE after migration)
```

---

### 2. RUSTSEC-2025-0134 Advisory (Blocked)

**Status:** Blocked by reqwest migration  
**Risk Level:** Medium

The `rustls-pemfile` crate is unmaintained. This is a transitive dependency from reqwest 0.11. Once reqwest is upgraded to 0.12+, this advisory will be automatically resolved.

**No action required** - this clears itself when item #1 is completed.

---

## 🟡 Medium Priority (Code Quality)

### 1. Hardcoded HMAC Secret in Gate

**Location:** [crates/fortify-gate/src/lib.rs#L112](../crates/fortify-gate/src/lib.rs#L112)  
**Effort:** Small (1-2 hours)

```rust
// TODO: Load secret from config
let secret = b"fortify-verification-secret-change-in-production";
```

**Problem:** HMAC secret for captcha verification tokens is hardcoded.

**Solution:**
1. Add `hmac_secret` field to `GateConfig` struct
2. Load from environment variable `FORTIFY_GATE_SECRET`
3. Fall back to config file `gate.secret` if not set
4. Fail startup if neither is set (no default in production)

**Implementation:**
```rust
// In config
pub struct GateConfig {
    pub hmac_secret: String,  // Required, no default
}

// At startup
let secret = std::env::var("FORTIFY_GATE_SECRET")
    .or_else(|_| config.hmac_secret.clone())
    .expect("FORTIFY_GATE_SECRET or gate.hmac_secret required");
```

---

### 2. Wipe Crypto Keys on Mirror Destroy

**Location:** [crates/fortify-orchestrator/src/lib.rs#L2493](../crates/fortify-orchestrator/src/lib.rs#L2493)  
**Effort:** Small (1-2 hours)

```rust
// TODO: Also wipe the keys from disk here
// let key_path = mirror.tor_data_dir.join("private_key");
// if key_path.exists() { std::fs::remove_file(key_path)?; }
```

**Problem:** When a mirror is permanently destroyed, its Tor private keys remain on disk.

**Security Impact:** Medium - Attacker with disk access could recover .onion addresses.

**Solution:**
1. Implement secure key wiping (overwrite before delete)
2. Also wipe `hostname`, `hs_ed25519_secret_key`, and `hs_ed25519_public_key`
3. Use `zeroize` crate for secure memory clearing

**Implementation:**
```rust
fn wipe_mirror_keys(tor_data_dir: &Path) -> std::io::Result<()> {
    let key_files = [
        "hostname",
        "hs_ed25519_secret_key", 
        "hs_ed25519_public_key",
    ];
    
    for file in key_files {
        let path = tor_data_dir.join("hidden_service").join(file);
        if path.exists() {
            // Overwrite with zeros before deleting
            let len = std::fs::metadata(&path)?.len() as usize;
            std::fs::write(&path, vec![0u8; len])?;
            std::fs::remove_file(&path)?;
        }
    }
    
    Ok(())
}
```

---

### 3. Implement Real CPU Monitoring

**Location:** [crates/fortify-orchestrator/src/lib.rs#L3708](../crates/fortify-orchestrator/src/lib.rs#L3708)  
**Effort:** Small (1-2 hours)

```rust
// TODO: Implement actual CPU monitoring via sys-info crate
// Currently returns a simulated value for development
```

**Problem:** CPU usage is simulated with random values instead of real monitoring.

**Impact:** Scaling decisions based on CPU load won't work correctly in production.

**Solution:**
The workspace already has `sysinfo = "0.30"` as a dependency.

```rust
use sysinfo::{System, CpuRefreshKind, RefreshKind};

async fn get_cpu_usage() -> f32 {
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_cpu(CpuRefreshKind::new().with_cpu_usage())
    );
    
    // First call returns 0, need to wait and refresh
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_cpu_usage();
    
    sys.global_cpu_usage()
}
```

---

### 4. Pagination Query Parameter Parsing

**Location:** [crates/fortify-http/src/admin.rs#L1919](../crates/fortify-http/src/admin.rs#L1919)  
**Effort:** Small (30 min)

```rust
let page = 1; // TODO: parse from query param
```

**Problem:** Sessions page always shows page 1, no pagination navigation works.

**Solution:**
```rust
fn parse_page_from_query(uri: &Uri) -> usize {
    uri.query()
        .and_then(|q| {
            q.split('&')
                .find_map(|pair| {
                    let mut parts = pair.split('=');
                    if parts.next() == Some("page") {
                        parts.next()?.parse().ok()
                    } else {
                        None
                    }
                })
        })
        .unwrap_or(1)
        .max(1)  // Minimum page 1
}

// Usage:
let page = parse_page_from_query(req.uri());
```

---

## 🟢 Lower Priority (Features from ROADMAP)

These are tracked in [ROADMAP.md](./ROADMAP.md) but listed here for completeness:

| Phase | Feature | Effort | Notes |
|-------|---------|--------|-------|
| 3.1 | Dynamic rate limits (server load) | Medium | Depends on CPU monitoring fix |
| 3.1 | Per-path rate limiting | Medium | Requires path pattern matching |
| 3.1 | Graduated slowdown | Small | Add delay tiers before hard block |
| 3.2 | Tor circuit rotation detection | Large | Complex Tor protocol analysis |
| 3.2 | Multi-circuit attack correlation | Large | Cross-node data sharing |
| 3.4 | Time-locked challenges | Small | Add timestamp validation |
| 4.1 | Automatic mirror spawning | Large | Full mirror lifecycle |
| 4.1 | DNS-like pointer system | Medium | Discovery mechanism |

---

## Recommended Action Order

### Immediate (This Sprint)
1. ✅ **Fix HMAC secret** - Quick security win
2. ✅ **Implement key wiping** - Security hardening
3. ✅ **Fix CPU monitoring** - Needed for production
4. ✅ **Fix pagination** - Minor UX improvement

### Short-term (Next Sprint)
5. 🔄 **Prepare hyper migration** - Add backports, run deprecation check
6. 🔄 **Migrate hyper incrementally** - One crate at a time

### Medium-term
7. 📋 Complete hyper 1.x migration
8. 📋 Upgrade reqwest to 0.12+
9. 📋 Remove RUSTSEC workarounds
10. 📋 Set `unmaintained = "warn"` in deny.toml

---

## Workarounds Audit

| Setting | Location | Status | Remove After |
|---------|----------|--------|--------------|
| `unmaintained = "none"` | deny.toml | ⚠️ Active | hyper migration |
| `ignore = ["RUSTSEC-2025-0134"]` | deny.toml | ⚠️ Active | reqwest upgrade |
| `ignore = ["RUSTSEC-2025-0134"]` | audit.toml | ⚠️ Active | reqwest upgrade |
| `wildcards = "allow"` | deny.toml | ✅ Permanent | N/A (workspace pattern) |
| `unused-ignored-advisory = "allow"` | deny.toml | ✅ Permanent | N/A (best practice) |

---

## References

- [hyper 1.0 Upgrade Guide](https://hyper.rs/guides/1/upgrading/)
- [reqwest Changelog](https://github.com/seanmonstar/reqwest/blob/master/CHANGELOG.md)
- [RUSTSEC-2025-0134](https://rustsec.org/advisories/RUSTSEC-2025-0134)
- [sysinfo crate docs](https://docs.rs/sysinfo)
