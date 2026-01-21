# 🔧 Component Reference

> **Individual Crate Documentation**

---

## Crate Overview

```
┌────────────────────────────────────────────────────────────────────────────┐
│                           FORTIFY CRATES                                    │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐ │
│   │                      FORTIFY-CORE                                    │ │
│   │           Shared types, trust system, sessions                       │ │
│   └──────────────────────────────┬──────────────────────────────────────┘ │
│                                  │                                         │
│        ┌────────────────┬────────┴────────┬────────────────┐              │
│        │                │                  │                │              │
│        ▼                ▼                  ▼                ▼              │
│   ┌─────────┐    ┌───────────┐    ┌────────────┐    ┌────────────┐       │
│   │  GATE   │    │   HTTP    │    │    NODE    │    │ CONTROLLER │       │
│   │ Captcha │    │   Proxy   │    │ Behavioral │    │  Tor Mgmt  │       │
│   └─────────┘    └───────────┘    └────────────┘    └────────────┘       │
│                                                                             │
│        │                │                  │                │              │
│        └────────────────┴────────┬─────────┴────────────────┘              │
│                                  │                                         │
│                                  ▼                                         │
│   ┌─────────────────────────────────────────────────────────────────────┐ │
│   │                      FORTIFY-ORCHESTRATOR                            │ │
│   │                 Mirror management, Tor coordination                  │ │
│   └─────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐ │
│   │                      FORTIFY-COMMUNITY                               │ │
│   │                   Threat intelligence sharing                        │ │
│   └─────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## fortify-core

**Path:** `crates/fortify-core/`

### Purpose
Foundation crate containing all shared types, the trust system, session management, configuration, and behavioral analysis engine.

### Modules

| Module | File | Purpose |
|--------|------|---------|
| `trust` | [trust.rs](../../crates/fortify-core/src/trust.rs) | Trust tiers, session tokens |
| `session` | [session.rs](../../crates/fortify-core/src/session.rs) | Session manager |
| `config` | [config.rs](../../crates/fortify-core/src/config.rs) | Configuration structures |
| `behavioral` | [behavioral.rs](../../crates/fortify-core/src/behavioral.rs) | Behavioral analysis |

### Key Types

```rust
// Trust tier enumeration
pub enum TrustTier {
    Burned,      // Permanently blocked
    Suspicious,  // Under scrutiny
    Unknown,     // New sessions
    Verified,    // Passed captcha
    Trusted,     // Long-term good behavior
}

// Session structure
pub struct Session {
    pub id: String,
    pub token: SessionToken,
    pub tier: TrustTier,
    pub created_at: SystemTime,
    pub request_count: u64,
    pub violation_count: u32,
    pub demotion_count: u32,
    pub is_demoted: bool,
    pub admin_override: Option<TrustTier>,
    pub is_killed: bool,
    pub is_banned: bool,
}

// Behavioral violation
pub enum ViolationType {
    PathEnumeration,
    FormSubmissionFlood,
    PayloadOverflow,
    MissingUserAgent,
    SuspiciousUserAgent,
    MissingReferer,
    AttackPathAccess,
    SequentialPathAccess,
    RapidRequests,
}
```

### Dependencies

- `tokio` - Async runtime
- `serde` - Serialization
- `hmac`, `sha2` - Token signing
- `base64` - Token encoding
- `uuid` - Session IDs

---

## fortify-gate

**Path:** `crates/fortify-gate/`

### Purpose
Entry point for new users. Handles CAPTCHA verification, proof-of-work challenges, and session token generation.

### Modules

| Module | File | Purpose |
|--------|------|---------|
| `lib` | [lib.rs](../../crates/fortify-gate/src/lib.rs) | Gate core, verification |
| `server` | [server.rs](../../crates/fortify-gate/src/server.rs) | HTTP endpoints |
| `captcha_types` | [captcha_types.rs](../../crates/fortify-gate/src/captcha_types.rs) | 7 captcha implementations |

### Captcha Types

| Type | Description | Difficulty |
|------|-------------|------------|
| `BmpText` | Distorted text image | Medium |
| `Emoji` | Emoji matching | Low |
| `Direction` | Arrow selection | Low |
| `Sequence` | Number ordering | Medium |
| `WordUnscramble` | Anagram solving | High |
| `ImageRotation` | Rotate to correct | Medium |
| `Silhouette` | Shape identification | Medium |

### Key Types

```rust
pub struct Gate {
    config: GateConfig,
    sessions: Arc<RwLock<HashMap<String, VerificationState>>>,
    rate_limiter: RateLimiter,
    captcha_type: CaptchaType,
    threat_captcha_type: Option<CaptchaType>,
}

pub struct CaptchaChallenge {
    pub id: String,
    pub captcha_type: CaptchaType,
    pub image_data: Vec<u8>,
    pub expected_answer: String,
    pub options: Option<Vec<String>>,
    pub created_at: SystemTime,
}

pub struct ProofOfWorkChallenge {
    pub id: String,
    pub prefix: String,
    pub difficulty: u32,
    pub created_at: SystemTime,
}
```

### Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/` | GET | Gate page with captcha |
| `/captcha` | GET | Get captcha image |
| `/verify` | POST | Submit captcha answer |
| `/pow` | GET | Get PoW challenge |
| `/pow/verify` | POST | Submit PoW solution |
| `/health` | GET | Health check |

---

## fortify-http

**Path:** `crates/fortify-http/`

### Purpose
HTTP reverse proxy handling verified user traffic. Routes requests to appropriate backend nodes, provides admin panel.

### Modules

| Module | File | Purpose |
|--------|------|---------|
| `lib` | [lib.rs](../../crates/fortify-http/src/lib.rs) | Proxy core |
| `admin` | [admin.rs](../../crates/fortify-http/src/admin.rs) | Admin dashboard |

### Key Types

```rust
pub struct HttpProxy {
    config: ProxyConfig,
    session_manager: Arc<SessionManager>,
    healthy_nodes: Vec<BackendNode>,
    threat_nodes: Vec<BackendNode>,
    metrics: ProxyMetrics,
}

pub struct BackendNode {
    pub id: String,
    pub address: String,
    pub port: u16,
    pub is_threat: bool,
    pub active_connections: AtomicU32,
}

pub struct AdminState {
    session_manager: Arc<SessionManager>,
    behavioral_analyzer: Arc<BehavioralAnalyzer>,
    node_statuses: Arc<RwLock<HashMap<String, NodeInfo>>>,
    mirror_statuses: Arc<RwLock<HashMap<String, MirrorInfo>>>,
    history_events: Arc<RwLock<VecDeque<HistoryEvent>>>,
}
```

### Routing Logic

```
┌────────────────────────────────────────────────────────────────────────────┐
│                         REQUEST ROUTING                                     │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   Incoming Request                                                         │
│         │                                                                   │
│         ▼                                                                   │
│   ┌─────────────┐                                                          │
│   │ Has Cookie? │                                                          │
│   └──────┬──────┘                                                          │
│          │                                                                  │
│     ┌────┴────┐                                                            │
│     │         │                                                             │
│    No        Yes                                                           │
│     │         │                                                             │
│     │         ▼                                                            │
│     │   ┌───────────┐                                                      │
│     │   │ Valid     │                                                      │
│     │   │ Token?    │                                                      │
│     │   └─────┬─────┘                                                      │
│     │         │                                                            │
│     │    ┌────┴────┐                                                       │
│     │    │         │                                                        │
│     │   No        Yes                                                      │
│     │    │         │                                                        │
│     │    │         ▼                                                       │
│     │    │   ┌───────────┐                                                 │
│     │    │   │ Is        │                                                 │
│     │    │   │ Demoted?  │                                                 │
│     │    │   └─────┬─────┘                                                 │
│     │    │         │                                                       │
│     │    │    ┌────┴────┐                                                  │
│     │    │    │         │                                                   │
│     │    │   Yes       No                                                  │
│     │    │    │         │                                                   │
│     ▼    ▼    ▼         ▼                                                  │
│   ┌──────────┐    ┌─────────────┐                                         │
│   │ REDIRECT │    │ Route to    │                                         │
│   │ TO GATE  │    │ Threat Node │                                         │
│   └──────────┘    └─────────────┘                                         │
│                         │                                                   │
│                   Yes   │   No                                             │
│                         ▼                                                   │
│                   ┌─────────────┐                                          │
│                   │ Route to    │                                          │
│                   │ Healthy     │                                          │
│                   │ Node        │                                          │
│                   └─────────────┘                                          │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## fortify-node

**Path:** `crates/fortify-node/`

### Purpose
Worker nodes that process user requests, perform behavioral analysis, and report violations. Can run in healthy or threat mode.

### Modules

| Module | File | Purpose |
|--------|------|---------|
| `lib` | [lib.rs](../../crates/fortify-node/src/lib.rs) | Node core, behavioral tracking |

### Key Types

```rust
pub struct Node {
    config: NodeConfig,
    mode: NodeMode,
    session_manager: Arc<SessionManager>,
    behavioral_analyzer: Arc<BehavioralAnalyzer>,
    backend_address: String,
    backend_port: u16,
}

pub enum NodeMode {
    Healthy,   // For trusted/verified users
    Threat,    // For suspicious/demoted users
}
```

### Node Modes

| Mode | Session Tiers | Behavior |
|------|---------------|----------|
| **Healthy** | Trusted, Verified, Unknown | Normal analysis, lenient thresholds |
| **Threat** | Suspicious, Demoted | Aggressive analysis, strict thresholds |

### Behavioral Tracking

The node performs per-request analysis:

1. **User-Agent Analysis** - Bot detection, missing headers
2. **Referer Analysis** - Missing/suspicious referers
3. **Path Analysis** - Attack patterns, enumeration
4. **Form Tracking** - Submission frequency
5. **Payload Analysis** - Size and content type

---

## fortify-orchestrator

**Path:** `crates/fortify-orchestrator/`

### Purpose
Manages Tor hidden services (mirrors), handles rotation, burn sequences, and coordinates distributed components.

### Modules

| Module | File | Purpose |
|--------|------|---------|
| `lib` | [lib.rs](../../crates/fortify-orchestrator/src/lib.rs) | Mirror management |
| `tor` | [tor.rs](../../crates/fortify-orchestrator/src/tor.rs) | Tor service integration |

### Key Types

```rust
pub struct Orchestrator {
    config: OrchestratorConfig,
    tor_service: TorService,
    mirrors: Arc<RwLock<HashMap<String, Mirror>>>,
    standby_mirrors: Arc<RwLock<Vec<Mirror>>>,
    compromise_signals: Arc<RwLock<HashMap<String, Vec<CompromiseSignal>>>>,
}

pub struct Mirror {
    pub id: String,
    pub onion_address: String,
    pub state: MirrorState,
    pub created_at: SystemTime,
    pub metrics: MirrorMetrics,
    pub compromise_score: f64,
}

pub enum MirrorState {
    Active,
    Standby,
    Paused,
    Burning,
    Burned,
}
```

### Mirror Lifecycle

```
┌────────────────────────────────────────────────────────────────────────────┐
│                       MIRROR STATE MACHINE                                  │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                              CREATE                                        │
│                                │                                           │
│                                ▼                                           │
│   ┌─────────────────────────────────────────────────────────────────────┐ │
│   │                          STANDBY                                     │ │
│   │           (Mirror exists but not serving traffic)                   │ │
│   └──────────────────────────────┬──────────────────────────────────────┘ │
│                                  │                                         │
│                           Activate                                        │
│                                  │                                         │
│                                  ▼                                         │
│   ┌─────────────────────────────────────────────────────────────────────┐ │
│   │                          ACTIVE                                      │ │
│   │              (Serving traffic, monitoring metrics)                   │ │
│   └────────────────────────────┬─┬──────────────────────────────────────┘ │
│                                │ │                                         │
│              ┌─────────────────┘ └─────────────────┐                       │
│              │                                     │                       │
│         Pause│                             Compromise                      │
│              │                             Detected                        │
│              ▼                                     │                       │
│   ┌──────────────────┐                            │                       │
│   │      PAUSED      │                            │                       │
│   │  (Temporarily    │                            │                       │
│   │   not serving)   │                            │                       │
│   └──────────────────┘                            │                       │
│              │                                     │                       │
│        Resume│                                     │                       │
│              │                                     │                       │
│              ▼                                     ▼                       │
│   ┌─────────────────────────────────────────────────────────────────────┐ │
│   │                         BURNING                                      │ │
│   │      (Showing redirect page, transitioning sessions away)           │ │
│   └──────────────────────────────┬──────────────────────────────────────┘ │
│                                  │                                         │
│                          Complete                                         │
│                                  │                                         │
│                                  ▼                                         │
│   ┌─────────────────────────────────────────────────────────────────────┐ │
│   │                          BURNED                                      │ │
│   │                (Tor service removed, archived)                       │ │
│   └─────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## fortify-controller

**Path:** `crates/fortify-controller/`

### Purpose
Manages Tor process, Vanguards addon integration, and coordinates security services.

### Modules

| Module | File | Purpose |
|--------|------|---------|
| `lib` | [lib.rs](../../crates/fortify-controller/src/lib.rs) | Controller core |
| `vanguards` | [vanguards.rs](../../crates/fortify-controller/src/vanguards.rs) | Vanguards management |

### Key Types

```rust
pub struct Controller {
    config: ControllerConfig,
    tor_manager: TorManager,
    vanguards_manager: Option<VanguardsManager>,
    service_manager: ServiceManager,
}

pub struct VanguardsManager {
    config: VanguardsConfig,
    process: Option<Child>,
    status: VanguardsStatus,
    last_rotation: Option<SystemTime>,
}

pub enum VanguardsStatus {
    Running,
    Stopped,
    Error(String),
    Rotating,
}
```

### Vanguards Integration

The Vanguards addon provides additional Tor security:

| Feature | Description |
|---------|-------------|
| **Rendguard** | Protects against rendezvous point attacks |
| **Bandguards** | Detects bandwidth-based attacks |
| **CBT Verify** | Circuit build time verification |
| **Layer Guards** | Additional guard layers |

---

## fortify-community

**Path:** `crates/fortify-community/`

### Purpose
(Planned) Threat intelligence sharing between Fortify instances. Allows sharing of known attack patterns, banned IPs, and compromise signals.

### Key Types

```rust
pub struct CommunityNetwork {
    config: CommunityConfig,
    peers: Vec<Peer>,
    shared_threats: SharedThreatDatabase,
}

pub struct CommunityConfig {
    pub enable_sharing: bool,
    pub network_key: String,
    pub bootstrap_peers: Vec<String>,
    pub share_violations: bool,
    pub share_compromises: bool,
}
```

### Planned Features

- [ ] Peer-to-peer threat sharing
- [ ] Encrypted communications
- [ ] Reputation system
- [ ] Distributed ban lists
- [ ] Real-time compromise alerts

---

*For complete function list, see [Functions.md](../Functions.md)*
