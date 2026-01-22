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

---

#### Detailed Migration Code

Below are the exact code changes needed for each pattern used in Fortify.

##### 1. Cargo.toml Changes (All Crates)

**Before:**
```toml
[dependencies]
hyper = { version = "0.14", features = ["server", "client", "http1", "tcp"] }
```

**After:**
```toml
[dependencies]
hyper = { version = "1.6", features = ["server", "client", "http1"] }
hyper-util = { version = "0.1", features = ["full"] }
http-body-util = "0.1"
tokio = { version = "1.35", features = ["full", "net"] }  # Need net for TcpListener
```

##### 2. Body Type Changes

**Before (0.14):**
```rust
use hyper::{Body, Response};

fn make_response() -> Response<Body> {
    Response::builder()
        .status(200)
        .body(Body::from("Hello"))
        .unwrap()
}

// Reading body
let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
```

**After (1.x):**
```rust
use http_body_util::{Full, BodyExt};
use hyper::body::Bytes;
use hyper::Response;

// For responses with known content
fn make_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(200)
        .body(Full::new(Bytes::from("Hello")))
        .unwrap()
}

// For empty responses
fn empty_response() -> Response<http_body_util::Empty<Bytes>> {
    Response::builder()
        .status(204)
        .body(http_body_util::Empty::new())
        .unwrap()
}

// Reading body - use BodyExt trait
use http_body_util::BodyExt;
let body_bytes = req.into_body().collect().await?.to_bytes();
```

##### 3. Server Changes (Critical - Used in 5 crates)

**Before (0.14) - `fortify-http/src/lib.rs`:**
```rust
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Server};

pub async fn run(&self) -> Result<()> {
    let make_svc = make_service_fn(move |_conn| {
        let state = state.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                handle_request(req, state.clone())
            }))
        }
    });

    let server = Server::bind(&self.bind_addr).serve(make_svc);
    server.await?;
    Ok(())
}
```

**After (1.x):**
```rust
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use http_body_util::Full;
use hyper::body::Bytes;

pub async fn run(&self) -> Result<()> {
    let listener = TcpListener::bind(&self.bind_addr).await?;
    tracing::info!("Listening on {}", self.bind_addr);

    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let state = self.state.clone();

        tokio::spawn(async move {
            let service = service_fn(move |req| {
                let state = state.clone();
                async move {
                    handle_request(req, state).await
                }
            });

            if let Err(e) = http1::Builder::new()
                .serve_connection(io, service)
                .await
            {
                tracing::error!("Connection error from {}: {}", remote_addr, e);
            }
        });
    }
}
```

##### 4. Client Changes

**Before (0.14):**
```rust
use hyper::{Client, Body};

let client = Client::new();
let resp = client.request(req).await?;
```

**After (1.x):**
```rust
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use http_body_util::Full;
use hyper::body::Bytes;

let client: Client<_, Full<Bytes>> = Client::builder(TokioExecutor::new())
    .build_http();
let resp = client.request(req).await?;
```

##### 5. Request Handler Signature Changes

**Before (0.14):**
```rust
async fn handle_request(
    req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    // ...
}
```

**After (1.x):**
```rust
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};

async fn handle_request(
    req: Request<Incoming>,  // Incoming is the new body type for requests
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    // ...
}
```

##### 6. Error Type Changes

**Before (0.14):**
```rust
type Result<T> = std::result::Result<T, hyper::Error>;
```

**After (1.x):**
```rust
// hyper::Error is now more specific - use Box<dyn Error> or custom error
use std::error::Error;
type BoxError = Box<dyn Error + Send + Sync>;
type Result<T> = std::result::Result<T, BoxError>;

// Or define a custom error enum
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("Hyper error: {0}")]
    Hyper(#[from] hyper::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

##### 7. Form Body Parsing (fortify-gate)

**Before (0.14):**
```rust
let body_bytes = hyper::body::to_bytes(req.body_mut()).await?;
let form_data = String::from_utf8_lossy(&body_bytes);
```

**After (1.x):**
```rust
use http_body_util::BodyExt;

let body_bytes = req.into_body().collect().await?.to_bytes();
let form_data = String::from_utf8_lossy(&body_bytes);
```

##### 8. Proxy Pass-Through (fortify-http/src/proxy.rs)

**Before (0.14):**
```rust
use hyper::{Body, Request, Response};

async fn proxy_request(
    mut req: Request<Body>,
    backend: &str,
) -> Result<Response<Body>, hyper::Error> {
    // Forward body as-is
    let client = Client::new();
    client.request(req).await
}
```

**After (1.x):**
```rust
use hyper::body::Incoming;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper_util::client::legacy::Client;

async fn proxy_request(
    req: Request<Incoming>,
    backend: &str,
) -> Result<Response<Incoming>, BoxError> {
    // Collect incoming body and forward
    let (parts, body) = req.into_parts();
    let body_bytes = body.collect().await?.to_bytes();
    
    let new_req = Request::from_parts(parts, Full::new(body_bytes));
    
    let client = Client::builder(TokioExecutor::new()).build_http();
    Ok(client.request(new_req).await?)
}
```

---

#### Fortify-Specific Migration Checklist

- [ ] **fortify-http/src/lib.rs** (Priority 1 - Main server)
  - [ ] Replace `Server::bind()` with `TcpListener` + loop
  - [ ] Change `make_service_fn` to per-connection service_fn
  - [ ] Update `handle_proxy_request` signature: `Request<Incoming>` → `Response<Full<Bytes>>`
  - [ ] Update `hyper::body::to_bytes()` calls to use `BodyExt::collect()`

- [ ] **fortify-http/src/admin.rs** (Priority 2 - Admin panel)
  - [ ] All `Response<Body>` → `Response<Full<Bytes>>`
  - [ ] Update body parsing in form handlers

- [ ] **fortify-gate/src/server.rs** (Priority 3 - Captcha gate)
  - [ ] Server builder changes
  - [ ] Form body parsing updates

- [ ] **fortify-node/src/server.rs** (Priority 4 - Node server)
  - [ ] Server builder changes
  - [ ] Proxy response handling

- [ ] **fortify-orchestrator/src/server.rs** (Priority 5 - Orchestrator)
  - [ ] Server builder changes
  - [ ] Client updates for health checks

- [ ] **fortify-controller/src/http.rs** (Priority 6 - Controller API)
  - [ ] Server builder changes

---

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

### 1. ~~Hardcoded HMAC Secret in Gate~~ ✅ COMPLETED

**Status:** ✅ Fixed in commit `961d38e`  
**Location:** [crates/fortify-gate/src/lib.rs](../crates/fortify-gate/src/lib.rs)

HMAC secret now loaded from `FORTIFY_GATE_SECRET` environment variable using lazy_static.
Falls back to default with warning if not set (for development only).

---

### 2. ~~Wipe Crypto Keys on Mirror Destroy~~ ✅ COMPLETED

**Status:** ✅ Fixed in commit `961d38e`  
**Location:** [crates/fortify-orchestrator/src/lib.rs](../crates/fortify-orchestrator/src/lib.rs)

Implemented `wipe_mirror_keys()` function that:
- Overwrites key files with zeros before deletion
- Wipes: `hostname`, `hs_ed25519_secret_key`, `hs_ed25519_public_key`, `private_key`
- Logs success/failure for audit trail

---

### 3. ~~Implement Real CPU Monitoring~~ ✅ COMPLETED

**Status:** ✅ Fixed in commit `961d38e`  
**Location:** [crates/fortify-orchestrator/src/lib.rs](../crates/fortify-orchestrator/src/lib.rs)

Replaced simulated random values with real sysinfo monitoring:
- Uses `sysinfo::System::global_cpu_info().cpu_usage()`
- Static System instance for efficiency
- Fallback to 25% if lock fails

---

### 4. ~~Pagination Query Parameter Parsing~~ ✅ COMPLETED

**Status:** ✅ Fixed in commit `961d38e`  
**Location:** [crates/fortify-http/src/admin.rs](../crates/fortify-http/src/admin.rs)

Added `parse_page_from_query()` function that:
- Parses `?page=N` from query string
- Clamps to valid range (1 to total_pages)
- Updated `render_sessions()` to accept URI parameter

---

## 🟢 Lower Priority (Features from ROADMAP)

These are tracked in [ROADMAP.md](./ROADMAP.md) but listed here for completeness:

| Phase | Feature | Effort | Notes |
|-------|---------|--------|-------|
| 3.1 | Dynamic rate limits (server load) | Medium | ✅ Unblocked - CPU monitoring now works |
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
1. ✅ **Fix HMAC secret** - Quick security win (DONE)
2. ✅ **Implement key wiping** - Security hardening (DONE)
3. ✅ **Fix CPU monitoring** - Needed for production (DONE)
4. ✅ **Fix pagination** - Minor UX improvement (DONE)

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
