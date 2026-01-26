# 🏗️ Architecture Overview

> **High-Level System Design of the Fortify Protection Layer**

---

## System Overview

Fortify is a **decentralized Tor hidden service protection layer** that acts as a proxy shield between attackers and the real hidden service. It implements:

- **Trust-based routing** - Sessions are routed based on their trust tier
- **Behavioral analysis** - Automated detection of bots and attackers
- **Multi-captcha verification** - JavaScript-free human verification
- **Mirror rotation** - Dynamic onion address rotation with burn capability
- **Vanguards protection** - Guard discovery attack mitigation

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                              FORTIFY ARCHITECTURE                                    │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│                              ┌─────────────────────┐                                │
│                              │     CONTROLLER      │                                │
│                              │  (Service Manager)  │                                │
│                              └──────────┬──────────┘                                │
│                                         │                                            │
│                    ┌────────────────────┼────────────────────┐                      │
│                    │                    │                    │                      │
│           ┌────────▼────────┐  ┌────────▼────────┐  ┌────────▼────────┐            │
│           │   ORCHESTRATOR  │  │   ORCHESTRATOR  │  │   ORCHESTRATOR  │            │
│           │   (Mirror Mgr)  │  │   (Mirror Mgr)  │  │   (Mirror Mgr)  │            │
│           └────────┬────────┘  └────────┬────────┘  └────────┬────────┘            │
│                    │                    │                    │                      │
│                    └────────────────────┼────────────────────┘                      │
│                                         │                                            │
│    ┌────────────────────────────────────▼────────────────────────────────────┐     │
│    │                              TOR NETWORK                                 │     │
│    │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌─────────────┐  │     │
│    │  │ Mirror 1.onion│  │ Mirror 2.onion│  │ Mirror 3.onion│  │ Standby    │  │     │
│    │  │   (Active)   │  │   (Active)   │  │   (Active)   │  │  (Paused)   │  │     │
│    │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └─────────────┘  │     │
│    └─────────┼─────────────────┼─────────────────┼───────────────────────────┘     │
│              │                 │                 │                                   │
│              └─────────────────┼─────────────────┘                                   │
│                                ▼                                                     │
│                   ┌────────────────────────┐                                        │
│                   │      HTTP PROXY        │◄────────────────┐                      │
│                   │   + ADMIN PANEL        │                 │                      │
│                   │  (Token Validation)    │                 │                      │
│                   └───────────┬────────────┘                 │                      │
│                               │                              │                      │
│           ┌───────────────────┼───────────────────┐          │                      │
│           │                   │                   │          │                      │
│           ▼                   ▼                   ▼          │                      │
│   ┌───────────────┐   ┌───────────────┐   ┌───────────────┐  │                      │
│   │     GATE      │   │    HEALTHY    │   │    THREAT     │  │                      │
│   │ (Verification)│   │    NODE(s)    │   │    NODE(s)    │──┘                      │
│   │               │   │               │   │               │  (Demoted)              │
│   └───────┬───────┘   └───────┬───────┘   └───────────────┘                         │
│           │                   │                                                      │
│           │                   ▼                                                      │
│           │           ┌───────────────────┐                                         │
│           └──────────►│   REAL SERVICE    │◄────────────────────────────────────────┤
│           (Token)     │  (Your App/Site)  │                                         │
│                       └───────────────────┘                                         │
│                                                                                      │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

---

## Request Flow

### New User Flow

```
┌────────────────────────────────────────────────────────────────────────────────────┐
│                               NEW USER REQUEST FLOW                                 │
├────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                     │
│  1. User connects to mirror.onion                                                   │
│     │                                                                               │
│     ▼                                                                               │
│  2. HTTP Proxy receives request                                                     │
│     │                                                                               │
│     ├─── No token/Invalid token ───────────────────────┐                           │
│     │                                                   ▼                           │
│     │                                    3. Redirect to GATE                        │
│     │                                       │                                       │
│     │                                       ▼                                       │
│     │                                    4. Cookie Compliance Check                 │
│     │                                       │                                       │
│     │                                       ├─── FAIL ──► Bot Block Page           │
│     │                                       │                                       │
│     │                                       ▼                                       │
│     │                                    5. Show Landing Page                       │
│     │                                       │                                       │
│     │                                       ▼                                       │
│     │                                    6. User clicks "Initialize Handshake"     │
│     │                                       │                                       │
│     │                                       ▼                                       │
│     │                                    7. Show Captcha Challenge                  │
│     │                                       │                                       │
│     │                                       ├─── WRONG ──► Retry (with delay)      │
│     │                                       │                                       │
│     │                                       ▼                                       │
│     │                                    8. Captcha Solved                          │
│     │                                       │                                       │
│     │                                       ▼                                       │
│     │                                    9. Issue VERIFIED Token                    │
│     │                                       │                                       │
│     │                                       ▼                                       │
│     │                                    10. Redirect to original URL              │
│     │                                       │                                       │
│     │◄──────────────────────────────────────┘                                      │
│     │                                                                               │
│     ▼                                                                               │
│  11. Valid token → Route to HEALTHY NODE                                           │
│     │                                                                               │
│     ▼                                                                               │
│  12. Forward to Real Service                                                        │
│                                                                                     │
└────────────────────────────────────────────────────────────────────────────────────┘
```

### Demoted User Flow (Behavioral Violation)

```
┌────────────────────────────────────────────────────────────────────────────────────┐
│                             DEMOTED USER REQUEST FLOW                               │
├────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                     │
│  1. User with VERIFIED/TRUSTED token makes request                                  │
│     │                                                                               │
│     ▼                                                                               │
│  2. HTTP Proxy validates token ✓                                                    │
│     │                                                                               │
│     ▼                                                                               │
│  3. Behavioral Analysis detects violation                                           │
│     │                                                                               │
│     ├─── Path Enumeration (sequential /page1, /page2, ...)                         │
│     ├─── Attack Path Access (/../etc/passwd)                                       │
│     ├─── Form Flood (>10 POSTs/minute)                                             │
│     ├─── Bot User-Agent (python-requests, curl, etc.)                              │
│     │                                                                               │
│     ▼                                                                               │
│  4. Violation recorded in session stats                                             │
│     │                                                                               │
│     ├─── Below threshold ──► Continue (warning logged)                             │
│     │                                                                               │
│     ▼                                                                               │
│  5. Threshold exceeded → DEMOTE to SUSPICIOUS                                       │
│     │                                                                               │
│     ▼                                                                               │
│  6. Set fortify_demoted=1 cookie                                                   │
│     │                                                                               │
│     ▼                                                                               │
│  7. Redirect to GATE                                                               │
│     │                                                                               │
│     ▼                                                                               │
│  8. Gate detects demoted cookie → Show "Hold Position" page                        │
│     │                                                                               │
│     ▼                                                                               │
│  9. HARD difficulty captcha + is_threat=true                                       │
│     │                                                                               │
│     ▼                                                                               │
│  10. Captcha 1 Solved                                                              │
│     │                                                                               │
│     ▼                                                                               │
│  11. captchas_remaining=1 → Generate 2nd Captcha                                   │
│     │                                                                               │
│     ▼                                                                               │
│  12. Captcha 2 Solved                                                              │
│     │                                                                               │
│     ▼                                                                               │
│  13. Re-issue VERIFIED Token (fresh, clears tier override)                         │
│     │                                                                               │
│     ▼                                                                               │
│  14. Return to HEALTHY Node pool                                                   │
│                                                                                     │
└────────────────────────────────────────────────────────────────────────────────────┘
```

---

## Component Responsibilities

### Controller (`fortify-controller`)

```
┌─────────────────────────────────────────────────────────────────────┐
│                         CONTROLLER                                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  RESPONSIBILITIES:                                                   │
│  ├── Service Lifecycle Management                                   │
│  │   ├── Spawn Gate, Proxy, Nodes, Orchestrators                    │
│  │   ├── Health checking (configurable interval)                    │
│  │   └── Auto-restart failed services                               │
│  │                                                                   │
│  ├── Auto-Scaling                                                    │
│  │   ├── Scale up at 80% CPU/Memory                                 │
│  │   └── Scale down at 20% CPU/Memory                               │
│  │                                                                   │
│  ├── Vanguards Management                                           │
│  │   ├── Start/Stop vanguards addon                                 │
│  │   ├── Monitor for guard discovery attacks                        │
│  │   └── Auto-restart on crash                                      │
│  │                                                                   │
│  └── Resource Monitoring                                             │
│      ├── CPU usage tracking                                          │
│      ├── Memory usage tracking                                       │
│      └── Metrics collection                                          │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Orchestrator (`fortify-orchestrator`)

```
┌─────────────────────────────────────────────────────────────────────┐
│                        ORCHESTRATOR                                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  RESPONSIBILITIES:                                                   │
│  ├── Mirror Lifecycle                                               │
│  │   ├── Create Tor hidden services (ADD_ONION/file-based)         │
│  │   ├── Maintain minimum active mirrors                            │
│  │   ├── Maintain standby mirrors (paused, ready)                  │
│  │   └── Delete burned mirrors                                      │
│  │                                                                   │
│  ├── Mirror Rotation                                                │
│  │   ├── Time-based rotation (configurable interval)               │
│  │   ├── Compromise-based rotation (score threshold)               │
│  │   └── Manual burn triggers (admin panel)                        │
│  │                                                                   │
│  ├── Compromise Detection                                           │
│  │   ├── Track request failures                                     │
│  │   ├── Monitor timing anomalies                                   │
│  │   ├── Calculate compromise scores                                │
│  │   └── Trigger burns at threshold                                │
│  │                                                                   │
│  ├── Tor Integration                                                │
│  │   ├── Control port communication                                 │
│  │   ├── Cookie authentication                                      │
│  │   └── PoW defense enablement                                    │
│  │                                                                   │
│  └── Persistence                                                    │
│      ├── Save mirror state to disk                                 │
│      └── Restore on restart                                         │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### HTTP Proxy (`fortify-http`)

```
┌─────────────────────────────────────────────────────────────────────┐
│                         HTTP PROXY                                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  RESPONSIBILITIES:                                                   │
│  ├── Token Validation                                               │
│  │   ├── Extract from cookies                                       │
│  │   ├── HMAC-SHA256 signature verification                         │
│  │   ├── Expiration checking                                        │
│  │   └── Trust tier extraction                                      │
│  │                                                                   │
│  ├── Request Routing                                                │
│  │   ├── No token → Gate (via API for landing pages)                │
│  │   ├── VERIFIED/TRUSTED → Healthy Nodes                           │
│  │   ├── SUSPICIOUS → Gate (re-verify via proxy)                    │
│  │   └── BURNED → Burned page                                       │
│  │                                                                   │
│  ├── Cached CAPTCHA Page Serving                                    │
│  │   ├── Calls Gate API for pre-rendered pages                      │
│  │   ├── Reduces per-request overhead for new visitors              │
│  │   ├── Falls back to full proxy if API fails                      │
│  │   └── Demoted users use full proxy (need 2-captcha flow)         │
│  │                                                                   │
│  ├── Behavioral Analysis                                            │
│  │   ├── Per-request analysis                                       │
│  │   ├── Violation tracking                                         │
│  │   ├── Auto-demotion on threshold                                │
│  │   └── Stats collection                                           │
│  │                                                                   │
│  ├── Admin Panel                                                    │
│  │   ├── Session management                                         │
│  │   ├── Node monitoring                                            │
│  │   ├── Mirror control                                             │
│  │   └── Configuration UI                                           │
│  │                                                                   │
│  ├── Backpressure Control                                           │
│  │   ├── Max concurrent connections                                 │
│  │   ├── Queue management                                           │
│  │   └── Graceful rejection                                         │
│  │                                                                   │
│  └── Metrics                                                        │
│      ├── Request counts                                             │
│      ├── Token validation stats                                     │
│      └── Backend error tracking                                     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Gate (`fortify-gate`)

```
┌─────────────────────────────────────────────────────────────────────┐
│                            GATE                                      │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  RESPONSIBILITIES:                                                   │
│  ├── Cookie Compliance                                              │
│  │   ├── Set test cookie                                            │
│  │   ├── Verify on return                                           │
│  │   └── Block clients without cookies                             │
│  │                                                                   │
│  ├── Captcha Generation                                             │
│  │   ├── 7 captcha types supported                                  │
│  │   ├── Difficulty levels (easy/medium/hard)                      │
│  │   ├── Random cycling option                                      │
│  │   └── Type-specific configuration                               │
│  │                                                                   │
│  ├── Pre-rendered Page API                                          │
│  │   ├── GET /gate/api/prerendered-page                             │
│  │   ├── Returns JSON with HTML, session_id, cookies                │
│  │   ├── Enables HTTP Proxy caching optimization                    │
│  │   └── Session creation still happens on Gate                     │
│  │                                                                   │
│  ├── Captcha Verification                                           │
│  │   ├── Case-insensitive text matching                            │
│  │   ├── Type-specific verification logic                          │
│  │   ├── Progressive delay on failures                             │
│  │   └── Multi-captcha for threat sessions                         │
│  │                                                                   │
│  ├── Proof-of-Work (disabled currently)                            │
│  │   ├── Challenge generation                                       │
│  │   ├── Nonce verification                                         │
│  │   └── Difficulty adjustment                                      │
│  │                                                                   │
│  ├── Token Issuance                                                 │
│  │   ├── Create VERIFIED token                                      │
│  │   ├── HMAC-SHA256 signing                                        │
│  │   └── Set in cookie                                              │
│  │                                                                   │
│  └── Rate Limiting                                                  │
│      ├── Per-IP limiting                                            │
│      └── Cleanup expired entries                                    │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Node (`fortify-node`)

```
┌─────────────────────────────────────────────────────────────────────┐
│                         NODE (Healthy/Threat)                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  RESPONSIBILITIES:                                                   │
│  ├── Request Processing                                             │
│  │   ├── Receive from HTTP Proxy                                    │
│  │   ├── Validate session token                                     │
│  │   └── Forward to backend                                         │
│  │                                                                   │
│  ├── Violation Detection                                            │
│  │   ├── Rate limit checking                                        │
│  │   ├── Request size validation                                    │
│  │   ├── Path validation                                            │
│  │   └── Pattern detection                                          │
│  │                                                                   │
│  ├── Session Actions                                                │
│  │   ├── Track violations per session                               │
│  │   ├── Demote on threshold                                        │
│  │   ├── Promote on good behavior                                   │
│  │   └── Redirect to Gate on demotion                              │
│  │                                                                   │
│  ├── Mode-Specific Behavior                                         │
│  │   ├── Healthy: Fast path, minimal inspection                    │
│  │   └── Threat: Deep inspection, strict limits                    │
│  │                                                                   │
│  └── Metrics                                                        │
│      ├── Request statistics                                         │
│      ├── Response times                                             │
│      └── Violation counts                                           │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Data Flow Summary

| Source | Destination | Data |
|--------|-------------|------|
| User → Mirror | Tor circuit | HTTP request |
| Mirror → Proxy | Internal | Request + headers |
| Proxy → Gate | Internal | Unknown sessions |
| Proxy → Node | Internal | Verified sessions |
| Gate → Proxy | Internal | Session token (cookie) |
| Node → Backend | Internal | Proxied request |
| Backend → User | Reverse path | HTTP response |
| Orchestrator → Tor | Control port | ADD_ONION commands |
| Controller → All | Internal | Health checks |
| Vanguards → Tor | Control port | Guard protection |

---

## Security Hardening

Fortify implements multiple layers of security hardening to prevent DoS attacks and ensure service resilience.

### Timeout Protection (Implemented January 2026)

All network-facing operations have explicit timeout configurations to prevent slow-loris attacks:

| Operation | Timeout | Component |
|-----------|---------|-----------|
| TCP Connection (handshake) | 10s | All network clients |
| HTTP Header Read | 30s | HTTP Proxy, Gate |
| Request (end-to-end) | 60s | Backend proxy |
| Tor Control Operations | 15s | Orchestrator |
| Max Buffer Size | 16KB | All HTTP servers |

**Implementation:**
- `connect_tor_control_with_timeout()` helper with 15-second limit
- Hyper server `header_read_timeout` configuration
- Backend proxy `timeout()` wrapper on all requests

### Safe Lock Helpers (Implemented January 2026)

Lock poisoning is handled gracefully to prevent cascading failures:

```rust
// Safe lock helpers in fortify-core
pub fn safe_lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T>;
pub fn safe_read<T>(rwlock: &RwLock<T>) -> RwLockReadGuard<'_, T>;
pub fn safe_write<T>(rwlock: &RwLock<T>) -> RwLockWriteGuard<'_, T>;
```

**Coverage:**
- fortify-http: 102 safe lock operations
- fortify-gate: 21 safe lock operations
- fortify-orchestrator: 77 safe lock operations

### Remaining Hardening (Planned)

| Task | Status | Impact |
|------|--------|--------|
| HTTP header parsing safety | Planned | Prevent panic on malformed headers |
| Token/session parsing safety | Planned | Prevent panic on crafted tokens |
| Fuzz testing infrastructure | Planned | Automated edge case discovery |
| Concurrency caps (semaphore gating) | Planned | Prevent connection exhaustion |
| Timeout jitter (±10-20%) | Planned | Prevent timing fingerprinting |

---

## Security Boundaries

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                            SECURITY BOUNDARIES                                   │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  ┌──────────────────────────────────────────────────────────────────────────┐  │
│  │                         EXTERNAL (Untrusted)                              │  │
│  │                                                                            │  │
│  │   • Tor Network                                                           │  │
│  │   • Mirror Onion Addresses                                                │  │
│  │   • User Connections                                                      │  │
│  │                                                                            │  │
│  └──────────────────────────────────────────────────────────────────────────┘  │
│                                      │                                          │
│                                      ▼                                          │
│  ┌──────────────────────────────────────────────────────────────────────────┐  │
│  │                           BOUNDARY LAYER                                  │  │
│  │                                                                            │  │
│  │   • HTTP Proxy (token validation)                                         │  │
│  │   • Gate (verification challenges)                                        │  │
│  │   • Behavioral Analysis                                                   │  │
│  │                                                                            │  │
│  └──────────────────────────────────────────────────────────────────────────┘  │
│                                      │                                          │
│                                      ▼                                          │
│  ┌──────────────────────────────────────────────────────────────────────────┐  │
│  │                          INTERNAL (Trusted)                               │  │
│  │                                                                            │  │
│  │   • Controller                                                            │  │
│  │   • Orchestrators                                                         │  │
│  │   • Nodes (Healthy/Threat)                                               │  │
│  │   • Admin Panel                                                           │  │
│  │                                                                            │  │
│  └──────────────────────────────────────────────────────────────────────────┘  │
│                                      │                                          │
│                                      ▼                                          │
│  ┌──────────────────────────────────────────────────────────────────────────┐  │
│  │                         PROTECTED (Isolated)                              │  │
│  │                                                                            │  │
│  │   • Real Hidden Service                                                   │  │
│  │   • Backend Application                                                   │  │
│  │   • Real Onion Address                                                    │  │
│  │                                                                            │  │
│  └──────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

*See [Functions.md](../Functions.md) for complete API reference*
