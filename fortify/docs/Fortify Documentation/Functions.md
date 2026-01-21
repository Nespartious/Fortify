# 📖 Fortify Functions Reference

> **Complete Index of Every Function, Process, Feature, Setting, and Configuration**

---

## Table of Contents

1. [Crate Overview](#crate-overview)
2. [fortify-core](#fortify-core)
3. [fortify-gate](#fortify-gate)
4. [fortify-http](#fortify-http)
5. [fortify-node](#fortify-node)
6. [fortify-orchestrator](#fortify-orchestrator)
7. [fortify-controller](#fortify-controller)
8. [fortify-community](#fortify-community)
9. [Configuration Reference](#configuration-reference)
10. [Tor Integration](#tor-integration)
11. [Admin Panel Features](#admin-panel-features)

---

## Crate Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           FORTIFY CRATE ARCHITECTURE                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│    ┌─────────────────┐                                                      │
│    │   Controller    │ ◄──── Manages all services, scaling, vanguards      │
│    └────────┬────────┘                                                      │
│             │                                                               │
│    ┌────────▼────────┐                                                      │
│    │   Orchestrator  │ ◄──── Mirror rotation, Tor HS creation/burn         │
│    └────────┬────────┘                                                      │
│             │                                                               │
│    ┌────────▼────────┐       ┌──────────────┐                              │
│    │   HTTP Proxy    │◄─────►│  Admin Panel │                              │
│    └────────┬────────┘       └──────────────┘                              │
│             │                                                               │
│    ┌────────┴────────┬───────────────┐                                     │
│    ▼                 ▼               ▼                                     │
│ ┌──────┐      ┌───────────┐   ┌───────────┐                               │
│ │ Gate │      │  Healthy  │   │  Threat   │                               │
│ │      │      │   Nodes   │   │   Nodes   │                               │
│ └──────┘      └───────────┘   └───────────┘                               │
│                                                                             │
│    ┌─────────────────────────────────────────┐                             │
│    │              fortify-core               │ ◄── Shared types & logic    │
│    └─────────────────────────────────────────┘                             │
│                                                                             │
│    ┌─────────────────────────────────────────┐                             │
│    │           fortify-community             │ ◄── P2P seed sharing        │
│    └─────────────────────────────────────────┘                             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## fortify-core

**Path:** `crates/fortify-core/src/`  
**Purpose:** Core types, trust system, session management, behavioral analysis

### Module Exports (`lib.rs`)
```rust
pub mod behavioral;  // Behavioral analysis engine
pub mod config;      // Configuration structures
pub mod session;     // Session management
pub mod trust;       // Trust tier system
```

---

### Trust System (`trust.rs` - 412 lines)

#### Enums

| Enum | Variants | Description |
|------|----------|-------------|
| `TrustError` | `TokenExpired`, `InvalidSignature`, `TokenBurned`, `InvalidTransition`, `Serialization`, `InvalidEncoding` | Error types for trust operations |
| `TrustTier` | `Burned(-2)`, `Suspicious(-1)`, `Unknown(0)`, `Verified(1)`, `Trusted(2)` | Session trust levels |

#### TrustTier Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `as_str()` | `fn as_str(&self) -> &'static str` | Get string representation ("burned", "suspicious", etc.) |
| `allows_access()` | `fn allows_access(&self) -> bool` | Returns `true` for Verified and Trusted tiers |
| `requires_gate()` | `fn requires_gate(&self) -> bool` | Returns `true` for Unknown and Suspicious |
| `can_promote()` | `fn can_promote(&self) -> bool` | Check if tier can be promoted |
| `can_demote()` | `fn can_demote(&self) -> bool` | Check if tier can be demoted |

#### SessionToken Struct

```rust
pub struct SessionToken {
    pub session_id: String,      // Unique session identifier
    pub trust_tier: TrustTier,   // Current trust level
    pub issued_at: u64,          // Unix timestamp of creation
    pub expires_at: u64,         // Unix timestamp of expiration
    pub signature: Vec<u8>,      // HMAC-SHA256 signature
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(session_id: String, trust_tier: TrustTier, lifetime_seconds: u64) -> Self` | Create new token |
| `sign()` | `fn sign(&mut self, secret_key: &[u8]) -> Result<()>` | Sign token with HMAC-SHA256 |
| `verify()` | `fn verify(&self, secret_key: &[u8]) -> Result<()>` | Verify token signature |
| `serialize_payload()` | `fn serialize_payload(&self) -> Result<String>` | Serialize for signing |
| `is_expired()` | `fn is_expired(&self) -> bool` | Check if token has expired |
| `is_valid()` | `fn is_valid(&self) -> bool` | Check if valid (not expired, not burned) |
| `encode()` | `fn encode(&self) -> Result<String>` | Base64 encode for cookie |
| `decode()` | `fn decode(encoded: &str) -> Result<Self>` | Decode from base64 |
| `time_until_expiry()` | `fn time_until_expiry(&self) -> Option<u64>` | Seconds until expiration |

#### Session Struct

```rust
pub struct Session {
    pub token: SessionToken,      // The session's token
    pub request_count: u64,       // Total requests made
    pub violation_count: u32,     // Violations recorded
    pub last_activity: u64,       // Last activity timestamp
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(token: SessionToken) -> Self` | Create from token |
| `record_request()` | `fn record_request(&mut self)` | Increment request count |
| `record_violation()` | `fn record_violation(&mut self)` | Increment violation count |
| `promote()` | `fn promote(&mut self) -> Result<()>` | Promote to higher tier |
| `demote()` | `fn demote(&mut self) -> Result<()>` | Demote to lower tier |
| `burn()` | `fn burn(&mut self)` | Permanently burn session |
| `should_demote()` | `fn should_demote(&self) -> bool` | Check if should be demoted |
| `should_burn()` | `fn should_burn(&self) -> bool` | Check if should be burned |
| `is_idle()` | `fn is_idle(&self, timeout_seconds: u64) -> bool` | Check if session is idle |

---

### Session Management (`session.rs`)

#### SessionManager Struct

```rust
pub struct SessionManager {
    sessions: HashMap<String, Session>,
    secret_key: Vec<u8>,
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(secret_key: Vec<u8>) -> Self` | Create new manager |
| `create_session()` | `fn create_session(&mut self, session_id: String) -> Session` | Create new session |
| `get_session()` | `fn get_session(&self, session_id: &str) -> Option<Session>` | Retrieve session |
| `update_session()` | `fn update_session(&mut self, session: Session)` | Update session |
| `remove_session()` | `fn remove_session(&mut self, session_id: &str)` | Remove session |
| `cleanup()` | `fn cleanup(&mut self, max_idle_seconds: u64)` | Remove idle sessions |
| `session_count()` | `fn session_count(&self) -> usize` | Total session count |
| `count_by_tier()` | `fn count_by_tier(&self, tier: TrustTier) -> usize` | Count sessions by tier |

---

### Behavioral Analysis (`behavioral.rs` - 957 lines)

#### Known Attack Paths Constant

```rust
pub const KNOWN_ATTACK_PATHS: &[(&str, &str, &str)]
```

Contains 25+ attack patterns including:
- Path traversal: `../`, `..\\`
- Config probing: `/.env`, `/.git`, `/.htaccess`
- CMS probing: `/wp-admin`, `/wp-login`, `/phpmyadmin`
- Exploit attempts: `/shell`, `/cmd`, `/eval`

#### ViolationType Enum

| Variant | Severity | Description |
|---------|----------|-------------|
| `SuspiciousUserAgent` | 2 | Bot/scraper User-Agent detected |
| `SuspiciousReferer` | 1 | External/suspicious referer |
| `PathEnumeration` | 2 | Sequential path scanning |
| `AttackPathAccess` | 3 | Known attack path accessed |
| `ResourceEnumeration` | 2 | Rapid unique path requests |
| `FormSubmissionFlood` | 2 | Excessive form submissions |
| `OversizedPayload` | 1 | Payload exceeds limit |
| `UndersizedPayload` | 1 | Suspiciously small payload |
| `AutomatedBehavior` | 3 | Pattern indicates bot |

#### BehaviorViolation Struct

```rust
pub struct BehaviorViolation {
    pub violation_type: ViolationType,
    pub timestamp: u64,
    pub details: String,
    pub severity: u8,
}
```

#### BehaviorStats Struct

```rust
pub struct BehaviorStats {
    pub requests_analyzed: u64,
    pub violations_by_type: HashMap<String, u64>,
    pub recent_violations: VecDeque<BehaviorViolation>,
    pub unique_paths_count: u64,
    pub form_submissions: u64,
    pub total_payload_bytes: u64,
    pub suspicious_ua_detected: bool,
    pub last_activity: u64,
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new() -> Self` | Create new stats |
| `record_violation()` | `fn record_violation(&mut self, violation: BehaviorViolation)` | Record a violation |
| `total_violations()` | `fn total_violations(&self) -> u64` | Sum of all violations |
| `severity_score()` | `fn severity_score(&self) -> u64` | Sum of severity scores |

#### BehaviorConfig Struct

```rust
pub struct BehaviorConfig {
    pub ua_analysis_enabled: bool,
    pub referer_analysis_enabled: bool,
    pub path_analysis_enabled: bool,
    pub enumeration_detection_enabled: bool,
    pub form_tracking_enabled: bool,
    pub payload_analysis_enabled: bool,
    pub max_unique_paths_per_minute: u32,      // Default: 60
    pub max_form_submissions_per_minute: u32,   // Default: 10
    pub max_payload_size: usize,                // Default: 10MB
    pub min_post_payload_size: usize,
    pub sequential_path_threshold: u32,         // Default: 5
    pub whitelisted_paths: Vec<String>,
    pub disabled_attack_paths: HashSet<String>,
    pub custom_whitelist_paths: Vec<String>,
    pub threat_demotion_threshold: u32,         // Default: 10
    pub threat_severity_threshold: u32,         // Default: 15
    pub violation_type_thresholds: HashMap<String, u32>,
    pub max_demotions_before_kill: u32,         // Default: 3
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `is_attack_path_enabled()` | `fn is_attack_path_enabled(&self, pattern: &str) -> bool` | Check if pattern enabled |
| `is_custom_whitelisted()` | `fn is_custom_whitelisted(&self, path: &str) -> bool` | Check whitelist |
| `is_path_whitelisted()` | `fn is_path_whitelisted(&self, path: &str) -> bool` | Legacy whitelist check |
| `should_demote_to_threat()` | `fn should_demote_to_threat(&self, stats: &BehaviorStats) -> bool` | Check demotion threshold |

#### SessionBehavior Struct

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(session_id: String, config: BehaviorConfig) -> Self` | Create analyzer |
| `analyze()` | `fn analyze(&mut self, req: &RequestMeta) -> Vec<BehaviorViolation>` | Analyze request |
| `get_stats()` | `fn get_stats(&self) -> &BehaviorStats` | Get current stats |
| `is_likely_automated()` | `fn is_likely_automated(&self) -> bool` | Check automation detection |
| `update_config()` | `fn update_config(&mut self, config: BehaviorConfig)` | Update config |
| `get_config()` | `fn get_config(&self) -> &BehaviorConfig` | Get current config |

#### BehaviorAnalyzer Struct (Global)

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(config: BehaviorConfig) -> Self` | Create global analyzer |
| `get_or_create_session()` | `fn get_or_create_session(&mut self, session_id: &str) -> &mut SessionBehavior` | Get/create session |
| `analyze()` | `fn analyze(&mut self, session_id: &str, req: &RequestMeta) -> Vec<BehaviorViolation>` | Analyze for session |
| `get_session_stats()` | `fn get_session_stats(&self, session_id: &str) -> Option<&BehaviorStats>` | Get session stats |
| `get_session()` | `fn get_session(&self, session_id: &str) -> Option<&SessionBehavior>` | Get session |
| `remove_session()` | `fn remove_session(&mut self, session_id: &str)` | Remove session |
| `cleanup()` | `fn cleanup(&mut self, max_idle_seconds: u64)` | Clean old sessions |
| `update_config()` | `fn update_config(&mut self, config: BehaviorConfig)` | Update global config |
| `get_global_stats()` | `fn get_global_stats(&self) -> &GlobalBehaviorStats` | Get global stats |
| `get_session_summary()` | `fn get_session_summary(&self) -> Vec<(String, u64)>` | Session violation summary |

---

### Configuration (`config.rs` - 167 lines)

#### FortifyConfig Struct

```rust
pub struct FortifyConfig {
    pub service: ServiceConfig,
    pub controller: ControllerConfig,
    pub orchestrator: OrchestratorConfig,
    pub gate: GateConfig,
    pub http_proxy: HttpProxyConfig,
    pub community: CommunityConfig,
    pub behavioral: BehavioralConfig,
    pub logging: LoggingConfig,
    pub security: SecurityConfig,
}
```

---

## fortify-gate

**Path:** `crates/fortify-gate/src/`  
**Purpose:** Verification system with captchas and proof-of-work

### Module Exports (`lib.rs` - 827 lines)

```rust
pub mod server;
pub mod bitmap;
pub mod captcha_types;
pub mod captcha_html;
```

### Errors

| Error | Description |
|-------|-------------|
| `RateLimitExceeded` | Too many requests |
| `InvalidCaptcha` | Wrong captcha solution |
| `InvalidProofOfWork` | Wrong PoW nonce |
| `ChallengeExpired` | Challenge timed out |
| `ChallengeNotFound` | Session not found |
| `QueueFull` | Max concurrent reached |
| `CookieComplianceFailed` | Client doesn't support cookies |
| `AdditionalCaptchaRequired` | Threat session needs more captchas |

### CaptchaChallenge Struct

```rust
pub struct CaptchaChallenge {
    pub challenge_id: String,
    pub text: String,           // 6 characters
    pub image_data: Vec<u8>,    // BMP image bytes
    pub created_at: u64,
    pub difficulty: CaptchaDifficulty,
    pub failed_attempts: u32,
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `generate()` | `fn generate() -> Self` | Generate medium difficulty |
| `generate_with_difficulty()` | `fn generate_with_difficulty(difficulty: CaptchaDifficulty) -> Self` | Generate with difficulty |
| `is_expired()` | `fn is_expired(&self, timeout_seconds: u64) -> bool` | Check expiration |
| `verify()` | `fn verify(&self, solution: &str) -> bool` | Verify solution (case-insensitive) |

### ProofOfWorkChallenge Struct

```rust
pub struct ProofOfWorkChallenge {
    pub challenge_id: String,
    pub challenge: Vec<u8>,     // 32 random bytes
    pub difficulty: u32,        // Leading zero bits required
    pub created_at: u64,
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(difficulty: u32) -> Self` | Create new PoW challenge |
| `verify()` | `fn verify(&self, nonce: u64) -> bool` | Verify nonce produces valid hash |
| `is_expired()` | `fn is_expired(&self, timeout_seconds: u64) -> bool` | Check expiration |

### VerificationState Struct

```rust
pub struct VerificationState {
    pub session_id: String,
    pub captcha_challenge: Option<CaptchaChallenge>,
    pub captcha_data: Option<CaptchaData>,
    pub captcha_type: CaptchaType,
    pub pow_challenge: Option<ProofOfWorkChallenge>,
    pub captcha_solved: bool,
    pub pow_solved: bool,
    pub created_at: u64,
    pub is_threat: bool,
    pub captchas_remaining: u8,  // 2 for threat, 1 for normal
    pub captchas_solved: u8,
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(session_id: String) -> Self` | Create new state |
| `is_complete()` | `fn is_complete(&self) -> bool` | All challenges solved? |

### RateLimiter Struct

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(max_requests: usize, window_seconds: u64) -> Self` | Create limiter |
| `check_rate_limit()` | `fn check_rate_limit(&self, key: &str) -> Result<()>` | Check if allowed |
| `cleanup()` | `fn cleanup(&self)` | Remove old entries |

### Gate Struct

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(bind_addr, max_concurrent, pow_difficulty, verification_timeout, session_manager, secret_key) -> Self` | Create gate |
| `start()` | `async fn start(&self) -> Result<()>` | Start server |
| `get_captcha_config()` | `fn get_captcha_config(&self) -> CaptchaConfig` | Get captcha config |
| `update_captcha_config()` | `fn update_captcha_config(&self, config: CaptchaConfig)` | Update config |
| `get_verification_timeout()` | `fn get_verification_timeout(&self) -> u64` | Get timeout |
| `create_verification()` | `fn create_verification(&self, session_id: String) -> Result<VerificationState>` | Create verification |
| `create_verification_with_difficulty()` | `fn create_verification_with_difficulty(&self, session_id: String, difficulty: CaptchaDifficulty) -> Result<VerificationState>` | Create with difficulty |
| `create_verification_with_type()` | `fn create_verification_with_type(&self, session_id, captcha_type, difficulty, is_threat) -> Result<VerificationState>` | Full control |
| `get_verification_state()` | `fn get_verification_state(&self, session_id: &str) -> Option<VerificationState>` | Get state |
| `verify_submission()` | `fn verify_submission(&self, session_id, captcha, pow_nonce) -> Result<String>` | Verify and issue token |
| `verify_captcha()` | `fn verify_captcha(&self, session_id: &str, solution: &str) -> Result<()>` | Verify captcha only |
| `verify_pow()` | `fn verify_pow(&self, session_id: &str, nonce: u64) -> Result<()>` | Verify PoW only |
| `issue_token()` | `fn issue_token(&self, session_id: &str, secret_key: &[u8]) -> Result<SessionToken>` | Issue session token |
| `regenerate_captcha()` | `fn regenerate_captcha(&self, session_id: &str, captcha_type: CaptchaType) -> Result<()>` | Generate new captcha |
| `get_failed_attempts()` | `fn get_failed_attempts(&self, session_id: &str) -> u32` | Get fail count |
| `get_captchas_remaining()` | `fn get_captchas_remaining(&self, session_id: &str) -> u8` | Captchas left |
| `get_captchas_solved()` | `fn get_captchas_solved(&self, session_id: &str) -> u8` | Captchas completed |
| `is_threat_session()` | `fn is_threat_session(&self, session_id: &str) -> bool` | Check threat status |
| `calculate_delay()` | `fn calculate_delay(&self, failed_attempts: u32) -> u64` | Progressive delay |
| `cleanup()` | `fn cleanup(&self)` | Clean expired states |

---

### Captcha Types (`captcha_types.rs` - 1004 lines)

#### CaptchaType Enum

| Type | Description |
|------|-------------|
| `BmpText` | Traditional text image captcha |
| `Emoji` | Select emoji matching description |
| `Direction` | Click arrow pointing in direction |
| `Sequence` | Complete the pattern |
| `WordUnscramble` | Unscramble letters |
| `ImageRotation` | Select correctly rotated image |
| `Silhouette` | Identify silhouette category |

| Method | Signature | Description |
|--------|-----------|-------------|
| `display_name()` | `fn display_name(&self) -> &'static str` | Human-readable name |
| `description()` | `fn description(&self) -> &'static str` | Brief description |
| `is_heavy()` | `fn is_heavy(&self) -> bool` | Computationally intensive? |
| `all()` | `fn all() -> Vec<CaptchaType>` | Get all types |

#### CaptchaConfig Struct

```rust
pub struct CaptchaConfig {
    pub gate_captcha_type: CaptchaType,
    pub threat_captcha_type: CaptchaType,
    pub threat_captcha_enabled: bool,
    pub random_cycling: bool,
    pub cycling_types: Vec<CaptchaType>,
    pub type_configs: HashMap<CaptchaType, CaptchaTypeConfig>,
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `get_captcha_type()` | `fn get_captcha_type(&self, is_threat: bool) -> CaptchaType` | Get type for context |
| `get_type_config()` | `fn get_type_config(&self, captcha_type: CaptchaType) -> CaptchaTypeConfig` | Get type config |

#### Challenge Types

Each has `generate()` and `verify(&self, solution: &str) -> bool` methods:

- `EmojiChallenge`
- `DirectionChallenge`
- `SequenceChallenge`
- `WordUnscrambleChallenge`
- `ImageRotationChallenge`
- `SilhouetteChallenge`

---

### Gate Server (`server.rs` - 1556 lines)

#### HTTP Endpoints

| Path | Method | Description |
|------|--------|-------------|
| `/Fortify` | GET | Landing page (new users) or demoted page |
| `/Fortify/Portcullis` | GET | Captcha challenge page |
| `/gate/verify` | POST | Submit captcha/PoW solution |
| `/gate/captcha/{id}` | GET | Captcha image |
| `/gate/admin/captcha-config` | POST | Update captcha config |

#### Server Functions

| Function | Description |
|----------|-------------|
| `handle_request()` | Main request router |
| `serve_cookie_blocked_page()` | Block clients without cookies |
| `serve_landing_page()` | New user landing page |
| `serve_demoted_page()` | Demoted user page with inline captcha |
| `serve_captcha_challenge()` | Captcha page for normal users |
| `verify_submission()` | Handle captcha/PoW verification |
| `serve_captcha_image()` | Serve BMP captcha image |
| `render_second_captcha_page()` | Second captcha for threat sessions |

---

## fortify-http

**Path:** `crates/fortify-http/src/`  
**Purpose:** HTTP proxy with admin panel

### Module Exports (`lib.rs` - 1329 lines)

```rust
pub mod admin;
pub mod middleware;
pub mod proxy;
pub mod routing;
```

### BackendNode Struct

```rust
pub struct BackendNode {
    pub address: String,
    pub healthy_mode: bool,
    pub weight: u32,
    pub active_connections: Arc<Mutex<usize>>,
    pub max_connections: usize,
}
```

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(address: String, healthy_mode: bool, max_connections: usize) -> Self` | Create node |
| `can_accept()` | `fn can_accept(&self) -> bool` | Check capacity |
| `acquire()` | `fn acquire(&self) -> bool` | Take connection slot |
| `release()` | `fn release(&self)` | Release connection slot |

### Metrics Struct

| Method | Signature | Description |
|--------|-----------|-------------|
| `record_request()` | `fn record_request(&mut self)` | Increment total |
| `record_allowed()` | `fn record_allowed(&mut self)` | Increment allowed |
| `record_denied()` | `fn record_denied(&mut self)` | Increment denied |
| `record_valid_token()` | `fn record_valid_token(&mut self)` | Increment valid |
| `record_invalid_token()` | `fn record_invalid_token(&mut self)` | Increment invalid |
| `record_backend_error()` | `fn record_backend_error(&mut self)` | Increment errors |

### HttpProxy Struct

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(bind_addr, max_concurrent, secret_key, session_manager, healthy_nodes, threat_nodes) -> Self` | Create proxy |
| `new_with_onions()` | `fn new_with_onions(..., healthy_onions, threat_onions, gate_address) -> Self` | Create with onions |
| `with_admin_state()` | `fn with_admin_state(..., admin_state) -> Self` | With shared state |
| `admin_state()` | `fn admin_state(&self) -> Arc<AdminState>` | Get admin state |
| `start()` | `async fn start(&self) -> Result<()>` | Start proxy |
| `get_metrics()` | `fn get_metrics(&self) -> Metrics` | Get metrics |
| `active_requests()` | `fn active_requests(&self) -> usize` | Current active |

### Request Processing

| Function | Description |
|----------|-------------|
| `handle_proxy_request()` | Main request handler |
| `process_request()` | Route and process request |
| `extract_token()` | Extract session token from cookies |
| `error_response()` | Generate error response |
| `is_mirror_paused()` | Check if mirror is paused |
| `serve_paused_mirror_page()` | Serve paused mirror redirect |

---

### Admin Panel (`admin.rs` - 3792 lines)

#### Admin Path
```rust
pub const ADMIN_PATH: &str = "/ctrl_8f7k3m9x2n4p1q6w5v0b8c";
```

#### AdminState Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new() -> Self` | Create state |
| `update_session()` | `fn update_session(&self, info: SessionInfo)` | Update session |
| `get_sessions()` | `fn get_sessions(&self) -> Vec<SessionInfo>` | Get all sessions |
| `get_session()` | `fn get_session(&self, id: &str) -> Option<SessionInfo>` | Get specific session |
| `record_page_load()` | `fn record_page_load(&self, session_id, path, method, status)` | Record page load |
| `record_session_event()` | `fn record_session_event(&self, session_id, event_type, description, source, reason)` | Record event |
| `set_session_tier()` | `fn set_session_tier(&self, session_id: &str, tier: &str)` | Set tier override |
| `get_tier_override()` | `fn get_tier_override(&self, session_id: &str) -> Option<String>` | Get override |
| `clear_tier_override()` | `fn clear_tier_override(&self, session_id: &str)` | Clear override |
| `ban_session()` | `fn ban_session(&self, session_id: &str)` | Ban session |
| `unban_session()` | `fn unban_session(&self, session_id: &str)` | Unban session |
| `update_node()` | `fn update_node(&self, info: NodeInfo)` | Update node info |
| `get_nodes()` | `fn get_nodes(&self) -> Vec<NodeInfo>` | Get all nodes |
| `update_mirror()` | `fn update_mirror(&self, info: MirrorInfo)` | Update mirror |
| `get_mirrors()` | `fn get_mirrors(&self) -> Vec<MirrorInfo>` | Get all mirrors |
| `get_behavior_config()` | `fn get_behavior_config(&self) -> BehaviorConfig` | Get behavior config |
| `set_behavior_config()` | `fn set_behavior_config(&self, config: BehaviorConfig)` | Set behavior config |
| `get_captcha_config()` | `fn get_captcha_config(&self) -> CaptchaConfig` | Get captcha config |
| `set_captcha_config()` | `fn set_captcha_config(&self, config: CaptchaConfig)` | Set captcha config |
| `record_traffic()` | `fn record_traffic(&self, bytes: u64, node_id: &str)` | Record traffic |
| `add_to_gate_queue()` | `fn add_to_gate_queue(&self, session_id: &str)` | Add to queue |
| `remove_from_gate_queue()` | `fn remove_from_gate_queue(&self, session_id: &str)` | Remove from queue |
| `record_mirror_request()` | `fn record_mirror_request(&self, mirror_addr: &str)` | Track mirror |

#### Admin Endpoints

| Path | Method | Description |
|------|--------|-------------|
| `/ctrl.../` | GET | Dashboard overview |
| `/ctrl.../sessions` | GET | Session list |
| `/ctrl.../sessions/{id}` | GET | Session detail |
| `/ctrl.../sessions/{id}/action` | POST | Session actions (ban, tier change) |
| `/ctrl.../nodes` | GET | Node list |
| `/ctrl.../mirrors` | GET | Mirror management |
| `/ctrl.../mirrors/{id}/action` | POST | Mirror actions (pause, burn) |
| `/ctrl.../behavioral` | GET | Behavioral config page |
| `/ctrl.../behavioral/config` | POST | Update behavioral config |
| `/ctrl.../captcha` | GET | Captcha config page |
| `/ctrl.../captcha/config` | POST | Update captcha config |
| `/ctrl.../traffic` | GET | Traffic analytics |

#### HistoryEventType Enum

| Type | Icon | Description |
|------|------|-------------|
| `PageRequest` | 📄 | Standard page request |
| `AdminTierChange` | 👮 | Admin changed tier |
| `AutoDemotion` | ⚠️ | System auto-demoted |
| `SessionBanned` | 🚫 | Session banned |
| `SessionUnbanned` | ✅ | Session unbanned |
| `SessionKilled` | 💀 | Session permanently killed |
| `CaptchaVerified` | 🔓 | Passed verification |
| `ViolationDetected` | 🚨 | Behavioral violation |

---

## fortify-node

**Path:** `crates/fortify-node/src/`  
**Purpose:** Backend node with request inspection

### Module Exports (`lib.rs` - 691 lines)

```rust
pub mod detection;
pub mod server;
```

### NodeMode Enum

| Mode | Max Req/Min | Timeout | Inspection |
|------|-------------|---------|------------|
| `Healthy` | 20 | 30s | Minimal |
| `Threat` | 10 | 10s | Deep |

| Method | Signature | Description |
|--------|-----------|-------------|
| `should_inspect_deeply()` | `fn should_inspect_deeply(&self) -> bool` | Deep inspection needed? |
| `max_requests_per_minute()` | `fn max_requests_per_minute(&self) -> u32` | Rate limit |
| `request_timeout()` | `fn request_timeout(&self) -> Duration` | Request timeout |

### ViolationType Enum

| Type | Severity | Description |
|------|----------|-------------|
| `RateLimitExceeded` | 1 | Too many requests |
| `MalformedRequest` | 2 | Bad request format |
| `SuspiciousPattern` | 3 | Suspicious content |
| `InvalidPath` | 1 | Invalid path |
| `OversizedRequest` | 2 | Request too large |

### NodeConfig Struct

```rust
pub struct NodeConfig {
    pub mode: NodeMode,
    pub bind_addr: SocketAddr,
    pub backend_address: String,
    pub gate_address: String,
    pub max_request_size: usize,        // Default: 10MB
    pub violation_threshold: u32,        // Default: 3
    pub promotion_threshold: u32,        // Default: 50
}
```

### Node Struct

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(config, session_manager, secret_key) -> Self` | Create node |
| `start()` | `async fn start(&self) -> Result<()>` | Start server |
| `process_request()` | `async fn process_request(&self, session_id, req) -> Result<Response>` | Process request |
| `check_violations()` | `fn check_violations(&self, session_id, req, session) -> Result<()>` | Check for violations |
| `forward_to_backend()` | `async fn forward_to_backend(&self, req) -> Result<Response>` | Forward request |
| `check_promotion()` | `fn check_promotion(&self, session_id: &str)` | Check for promotion |
| `redirect_to_gate()` | `fn redirect_to_gate(&self) -> Response` | Redirect to Gate |
| `error_response()` | `fn error_response(&self, status, message) -> Response` | Generate error |

### NodeMetrics Struct

| Field | Type | Description |
|-------|------|-------------|
| `requests_total` | `u64` | Total requests |
| `requests_forwarded` | `u64` | Successfully forwarded |
| `requests_blocked` | `u64` | Blocked requests |
| `violations_detected` | `u64` | Total violations |
| `sessions_demoted` | `u64` | Sessions demoted |
| `sessions_promoted` | `u64` | Sessions promoted |
| `backend_errors` | `u64` | Backend errors |
| `average_response_time_ms` | `f64` | Average response time |

---

## fortify-orchestrator

**Path:** `crates/fortify-orchestrator/src/`  
**Purpose:** Mirror lifecycle management

### Module Exports (`lib.rs` - 929 lines)

```rust
pub mod detection;
pub mod mirror;
pub mod server;
pub mod tor;
```

### MirrorState Enum

| State | Can Serve? | Should Replace? | Description |
|-------|-----------|-----------------|-------------|
| `Spawning` | No | No | Being created |
| `Active` | Yes | No | Serving traffic |
| `Paused` | No | No | Admin paused |
| `Suspicious` | Yes | No | Under suspicion |
| `Burning` | No | Yes | Being burned |
| `Burned` | No | Yes | Fully burned |

### CompromiseSignal Struct

```rust
pub struct CompromiseSignal {
    pub signal_type: SignalType,
    pub severity: f32,
    pub timestamp: u64,
    pub description: String,
}
```

### SignalType Enum

- `UnusualTraffic`
- `TimingAnomaly`
- `RepeatedFailures`
- `MemoryExhaustion`
- `NetworkAnomaly`

### MirrorMetrics Struct

| Field | Type | Description |
|-------|------|-------------|
| `requests_total` | `u64` | Total requests |
| `requests_failed` | `u64` | Failed requests |
| `bytes_transferred` | `u64` | Total bytes |
| `uptime_seconds` | `u64` | Uptime |
| `last_request_time` | `Option<u64>` | Last activity |
| `average_response_time_ms` | `f64` | Avg response |
| `compromise_score` | `f32` | 0.0-1.0 score |

| Method | Signature | Description |
|--------|-----------|-------------|
| `record_request()` | `fn record_request(&mut self, success, response_time_ms, bytes)` | Record request |
| `failure_rate()` | `fn failure_rate(&self) -> f64` | Calculate failure rate |
| `is_healthy()` | `fn is_healthy(&self) -> bool` | Check health |

### Mirror Struct

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(id: String, tor_data_dir: PathBuf) -> Self` | Create mirror |
| `activate()` | `fn activate(&mut self, onion_address: String)` | Activate mirror |
| `activate_as_standby()` | `fn activate_as_standby(&mut self, onion_address: String)` | Activate paused |
| `add_signal()` | `fn add_signal(&mut self, signal: CompromiseSignal)` | Add compromise signal |
| `burn()` | `fn burn(&mut self)` | Start burn process |
| `complete_burn()` | `fn complete_burn(&mut self)` | Complete burn |
| `age_seconds()` | `fn age_seconds(&self) -> u64` | Get age |

### OrchestratorConfig Struct

```rust
pub struct OrchestratorConfig {
    pub min_mirrors: usize,              // Default: 2
    pub max_mirrors: usize,              // Default: 5
    pub standby_mirrors: usize,          // Default: 2
    pub rotation_interval_seconds: u64,   // Default: 3600 (1 hour)
    pub burn_threshold: f32,             // Default: 0.7
    pub tor_data_dir: PathBuf,
    pub gate_address: String,
    pub public_bind_addr: String,
    pub proxy_port: u16,
    pub tor_control_addr: Option<String>,
    pub tor_cookie_path: Option<PathBuf>,
}
```

### Orchestrator Struct

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(config: OrchestratorConfig) -> Self` | Create orchestrator |
| `start()` | `async fn start(&self) -> Result<()>` | Start orchestrator |
| `load_mirrors()` | `async fn load_mirrors(&self)` | Load existing mirrors |
| `ensure_minimum_mirrors()` | `async fn ensure_minimum_mirrors(&self) -> Result<()>` | Spawn minimum |
| `start_rotation_task()` | `fn start_rotation_task(&self)` | Start rotation |
| `start_monitoring_task()` | `fn start_monitoring_task(&self)` | Start monitoring |

---

### Tor Integration (`tor.rs` - 635 lines)

#### TorService Struct

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(control_addr: Option<String>, cookie_path: Option<PathBuf>) -> Self` | Create service |
| `create_hidden_service()` | `fn create_hidden_service(&self, mirror, target_port) -> Result<String>` | Create HS |
| `delete_hidden_service()` | `fn delete_hidden_service(&self, service_id: &str) -> Result<()>` | Delete HS |
| `authenticate()` | `fn authenticate(&mut self, stream, cookie_path) -> Result<()>` | Authenticate to Tor |
| `run_command()` | `fn run_command(&mut self, stream, command) -> Result<String>` | Run Tor command |

#### PoW Strategy

1. Try `ADD_ONION` with `Flags=PoWDefensesEnabled` (Tor 0.4.9.2+)
2. Fallback to file-based PoW via torrc include (Tor 0.4.8+)

---

## fortify-controller

**Path:** `crates/fortify-controller/src/`  
**Purpose:** Service management and scaling

### Module Exports (`lib.rs` - 691 lines)

```rust
pub mod config;
pub mod resource;
pub mod scaling;
pub mod service;
pub mod tor;
pub mod vanguards;
```

### Controller Struct

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(config: ControllerConfig) -> Self` | Create controller |
| `start()` | `async fn start(&self) -> Result<()>` | Start controller |
| `start_http_api()` | `fn start_http_api(&self) -> Result<()>` | Start HTTP API |
| `start_initial_services()` | `async fn start_initial_services(&self) -> Result<()>` | Start services |
| `start_vanguards()` | `async fn start_vanguards(&self) -> Result<()>` | Start vanguards |
| `start_monitoring()` | `async fn start_monitoring(&self)` | Start monitoring |
| `start_scaling()` | `async fn start_scaling(&self)` | Start auto-scaling |
| `shutdown()` | `async fn shutdown(&self) -> Result<()>` | Graceful shutdown |
| `get_metrics()` | `async fn get_metrics(&self) -> ControllerMetrics` | Get metrics |

### ControllerMetrics Struct

```rust
pub struct ControllerMetrics {
    pub services_running: usize,
    pub services_failed: usize,
    pub services_restarted: usize,
    pub scaling_events: usize,
    pub cpu_usage_percent: f32,
    pub memory_usage_mb: u64,
    pub total_memory_mb: u64,
    pub vanguards_status: String,
    pub vanguards_uptime_secs: Option<u64>,
}
```

### ServiceType Enum

- `Gate`
- `Node`
- `HttpProxy`
- `Orchestrator`

---

### Vanguards Integration (`vanguards.rs` - 408 lines)

#### VanguardsStatus Enum

- `NotConfigured`
- `Starting`
- `Running`
- `Failed`
- `Stopped`

#### VanguardsConfig Struct

```rust
pub struct VanguardsConfig {
    pub config_path: String,
    pub state_path: String,
    pub log_path: String,
    pub tor_control_addr: String,
    pub tor_control_port: u16,
    pub enabled: bool,
    pub layer2_guards: u8,     // Default: 4
    pub layer3_guards: u8,     // Default: 8
    pub circ_max_age_hours: u32,
    pub circ_max_megabytes: u32,
}
```

#### VanguardsManager Struct

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(config: VanguardsConfig) -> Self` | Create manager |
| `with_defaults()` | `fn with_defaults() -> Self` | Default config |
| `is_available()` | `fn is_available() -> bool` | Check if installed |
| `find_vanguards_path()` | `fn find_vanguards_path() -> Option<(String, Vec<String>)>` | Find binary |
| `generate_config()` | `fn generate_config(&self) -> Result<()>` | Generate config |
| `start()` | `fn start(&mut self) -> Result<()>` | Start vanguards |
| `stop()` | `fn stop(&mut self) -> Result<()>` | Stop vanguards |
| `restart()` | `fn restart(&mut self) -> Result<()>` | Restart |
| `is_alive()` | `fn is_alive(&self) -> bool` | Check if running |
| `status()` | `fn status(&self) -> VanguardsStatus` | Get status |
| `uptime_secs()` | `fn uptime_secs(&self) -> Option<u64>` | Get uptime |
| `check_for_attacks()` | `fn check_for_attacks(&self) -> Vec<String>` | Check for alerts |

---

## fortify-community

**Path:** `crates/fortify-community/src/`  
**Purpose:** P2P seed sharing network

### Module Exports (`lib.rs`)

```rust
pub mod crypto;
pub mod discovery;
pub mod registry;
pub mod server;
```

### CommunityConfig Struct

```rust
pub struct CommunityConfig {
    pub enabled: bool,                       // Default: false (opt-in)
    pub bind_addr: String,
    pub max_seeds: usize,                    // Default: 100
    pub seed_ttl: Duration,                  // Default: 7 days
    pub discovery_enabled: bool,
    pub max_discovery_hops: usize,           // Default: 3
    pub share_rate_limit: usize,             // Default: 10 req/min
}
```

### CommunityNetwork Struct

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn new(config: CommunityConfig, keypair: KeyPair) -> Self` | Create network |
| `start()` | `async fn start(&self) -> Result<()>` | Start network |
| `add_seed()` | `async fn add_seed(&self, seed: Seed) -> Result<()>` | Add seed |
| `get_seeds()` | `async fn get_seeds(&self) -> Vec<Seed>` | Get active seeds |
| `discover_peers()` | `async fn discover_peers(&self, max_results: usize) -> Result<Vec<Seed>>` | Discover peers |
| `verify_seed()` | `fn verify_seed(&self, seed: &Seed) -> bool` | Verify signature |
| `sign_seed()` | `fn sign_seed(&self, seed: &mut Seed)` | Sign seed |
| `get_metrics()` | `async fn get_metrics(&self) -> CommunityMetrics` | Get metrics |

### CommunityMetrics Struct

```rust
pub struct CommunityMetrics {
    pub seeds_total: usize,
    pub seeds_active: usize,
    pub seeds_expired: usize,
    pub discoveries_performed: usize,
    pub peers_discovered: usize,
    pub signatures_verified: usize,
    pub signatures_failed: usize,
}
```

---

## Configuration Reference

### Main Configuration (`fortify.example.toml`)

```toml
[service]
real_onion_address = "http://xxx.onion"
real_service_port = 80

[controller]
bind_address = "127.0.0.1:7000"
max_orchestrators = 5
max_healthy_nodes = 10
max_threat_nodes = 5
scale_up_threshold = 0.8
scale_down_threshold = 0.2

[orchestrator]
bind_address = "127.0.0.1:8080"
max_connections_per_minute = 100
max_failed_challenges = 50
rotation_interval_hours = 24
tor_control_port = "127.0.0.1:9051"
tor_socks_port = "127.0.0.1:9050"
tor_control_addr = "127.0.0.1:9151"
tor_cookie_path = "/tmp/fortify/tor/data/control_auth_cookie"

[gate]
bind_address = "127.0.0.1:8081"
max_concurrent_verifications = 10
verification_timeout_seconds = 300
captcha_difficulty = "medium"
pow_difficulty = 20
token_lifetime_seconds = 3600

[http_proxy]
bind_address = "127.0.0.1:8082"
max_concurrent_connections = 1000
connection_timeout_seconds = 30
max_request_size_bytes = 10485760
queue_size = 100
reject_when_full = true

[node]
bind_base = "127.0.0.1:9100"
backend_address = "http://127.0.0.1:9000"

[community]
enabled = false
mode = "standalone"

[logging]
level = "info"
output = "syslog"
log_file = "/var/log/fortify/fortify.log"

[security]
drop_privileges = true
secure_memory = true
```

---

## Tor Integration

### Tor Control Commands Used

| Command | Purpose |
|---------|---------|
| `AUTHENTICATE` | Authenticate to control port |
| `ADD_ONION` | Create new hidden service |
| `DEL_ONION` | Delete hidden service |
| `SIGNAL NEWNYM` | Get new circuits |
| `SIGNAL RELOAD` | Reload configuration |

### Tor PoW Methods

1. **ADD_ONION PoW** (Tor 0.4.9.2+)
   ```
   ADD_ONION NEW:ED25519-V3 Port=80,127.0.0.1:8080 Flags=Detach,PoWDefensesEnabled
   ```

2. **File-based PoW** (Tor 0.4.8+)
   ```
   HiddenServiceDir /path/to/hs
   HiddenServicePort 80 127.0.0.1:8080
   HiddenServicePoWDefensesEnabled 1
   ```

### Vanguards Protection

```
[Vanguards]
num_layer2_guards = 4
num_layer3_guards = 8
min_layer2_lifetime_days = 1
max_layer2_lifetime_days = 30
```

---

## Admin Panel Features

### Dashboard Overview
- Total sessions count
- Active sessions by tier
- Recent violations
- Mirror status
- Node health

### Session Management
- View all sessions
- View session detail/history
- Change session tier
- Ban/unban sessions
- Kill sessions (repeat offenders)

### Mirror Management
- View all mirrors
- Pause/unpause mirrors
- Manual burn triggers
- Create standby mirrors
- Monitor compromise scores

### Behavioral Configuration
- Toggle detection features
- Adjust thresholds
- Enable/disable attack paths
- Configure whitelists
- Set demotion thresholds

### Captcha Configuration
- Select captcha types
- Configure per-type settings
- Enable random cycling
- Set threat vs normal types

### Traffic Analytics
- Requests per time period
- Bytes transferred
- Per-node statistics
- Mirror traffic distribution

---

## Session Continuity System (Planned - Future Feature)

> **Status:** Planned - Medium Priority. Enables seamless session restoration for paused VMs.

### SessionHistoryRecord Struct (Planned)

```rust
/// Minimal record for session continuity across token expiry
pub struct SessionHistoryRecord {
    pub session_id: String,           // UUID of original session
    pub last_trust_tier: TrustTier,   // Last known trust status
    pub demotion_count: u32,          // Carries over to new session
    pub was_killed: bool,             // Permanent kill flag
    pub was_burned: bool,             // Permanent burn flag
    pub created_at: u64,              // Original session creation time
    pub last_seen_at: u64,            // Last activity timestamp
    pub expires_at: u64,              // 7 days from last_seen
    pub successor_id: Option<String>, // Link to new session if continued
}
```

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | `String` | UUID of original session |
| `last_trust_tier` | `TrustTier` | Verified, Trusted, Suspicious, etc. |
| `demotion_count` | `u32` | Number of times demoted (transfers to new session) |
| `was_killed` | `bool` | If true, continuity is DENIED |
| `was_burned` | `bool` | If true, continuity is DENIED |
| `created_at` | `u64` | Unix timestamp of session creation |
| `last_seen_at` | `u64` | Unix timestamp of last request |
| `expires_at` | `u64` | Unix timestamp when history expires (7 days) |
| `successor_id` | `Option<String>` | If session was continued, links to new session |

### Session Continuity Methods (Planned)

| Method | Signature | Description |
|--------|-----------|-------------|
| `SessionHistory::new()` | `fn new(db_path: &str) -> Self` | Create/open history database |
| `SessionHistory::record()` | `fn record(&self, session: &Session)` | Store session in history |
| `SessionHistory::lookup()` | `fn lookup(&self, session_id: &str) -> Option<SessionHistoryRecord>` | Find session by ID |
| `SessionHistory::continue_session()` | `fn continue_session(&self, old_id: &str) -> Result<Session, Error>` | Create new session from history |
| `SessionHistory::link()` | `fn link(&self, old_id: &str, new_id: &str)` | Link old session to successor |
| `SessionHistory::cleanup()` | `fn cleanup(&self)` | Remove expired records |
| `SessionHistoryRecord::is_valid()` | `fn is_valid(&self) -> bool` | Check if record is valid for continuity |
| `SessionHistoryRecord::can_continue()` | `fn can_continue(&self) -> bool` | Check killed/burned status |

### Session Continuity Configuration (Planned)

```toml
[session_continuity]
enabled = true                      # Enable session continuity
max_age_days = 7                    # Maximum history retention (days)
storage_backend = "sqlite"          # sqlite | sled | memory
database_path = "/var/lib/fortify/sessions.db"

[session_continuity.transfer]
transfer_tier = true                # Transfer trust tier to new session
transfer_demotion_count = true      # Transfer demotion count
reset_violation_count = true        # Reset violations (fresh start)
deny_if_killed = true               # Block killed sessions from continuing
deny_if_burned = true               # Block burned sessions from continuing

[session_continuity.cleanup]
run_interval_hours = 24             # Cleanup frequency
vacuum_on_cleanup = true            # SQLite vacuum after cleanup
```

---

## Defensive Mechanisms (Complete - Phase 3)

### Dynamic Rate Limiting

```rust
/// Dynamic rate limiter that adjusts based on system load
pub struct DynamicRateLimiter {
    base_limits: HashMap<TrustTier, u32>,  // Base requests per minute
    current_multiplier: f32,                // 0.1 to 1.0
    load_monitor: SystemLoadMonitor,
}
```

**Load-Based Multipliers:**

| System Load | Multiplier | Effect |
|-------------|------------|--------|
| 0-50% CPU | 1.0x | Normal limits |
| 50-70% CPU | 0.75x | 25% reduction |
| 70-85% CPU | 0.5x | 50% reduction |
| 85-95% CPU | 0.25x | 75% reduction |
| 95%+ CPU | 0.1x | Survival mode |

### Bandwidth Throttling

```rust
/// Token bucket bandwidth limiter per session
pub struct BandwidthThrottler {
    limits: HashMap<TrustTier, BytesPerMinute>,
    buckets: HashMap<String, TokenBucket>,  // session_id → bucket
}
```

**Per-Tier Limits:**

| Trust Tier | Bandwidth Limit | Response Delay |
|------------|-----------------|----------------|
| Trusted | Unlimited | None |
| Verified | 10 MB/min | None |
| Unknown | 5 MB/min | None |
| Suspicious | 1 MB/min | +500ms |
| Demoted | 500 KB/min | +1000ms |

### Honeypot Endpoints

```rust
/// Honeypot trap configuration
pub struct HoneypotEndpoint {
    pub path: String,           // e.g., "/admin", "/.env"
    pub trap_type: TrapType,    // AdminTrap, FileTrap, ApiTrap
    pub response: TrapResponse, // FakeLogin, FakeEnv, FakeJson
    pub tarpit_seconds: u32,    // Slow-drip response time
    pub immediate_action: TrapAction,  // Demote, Burn, Flag
}

pub enum TrapType {
    AdminTrap,      // Fake admin panels
    FileTrap,       // Fake sensitive files (.env, .git)
    ApiTrap,        // Fake API endpoints
    DirectoryTrap,  // Infinite directory listings
    FormTrap,       // Hidden form fields (bot detection)
}

pub enum TrapAction {
    Flag,           // Just flag for review
    Demote,         // Immediate demotion
    Burn,           // Immediate burn
    Tarpit,         // Slow-drip response
}
```

**Honeypot Configuration:**

```toml
[honeypots]
enabled = true
log_attempts = true
immediate_demotion = true

[[honeypots.endpoints]]
path = "/admin"
type = "admin_trap"
response = "fake_login"
tarpit_seconds = 30

[[honeypots.endpoints]]
path = "/.env"
type = "file_trap"
immediate_burn = true

[honeypots.hidden_fields]
enabled = true
field_name = "website_url"
fill_action = "demote"
```

---

## Fast-Pass System (Planned - Future Feature)

> **Status:** Planned - Low Priority. These structures are documented for future implementation reference.

### FastPassProfile Struct (Planned)

```rust
/// Persistent identity profile for Fast-Pass users
pub struct FastPassProfile {
    pub id: String,                          // UUID
    pub key_fingerprint: String,             // PGP key fingerprint (SHA-256)
    pub tier: FastPassTier,                  // Squire or Knight
    pub created_at: SystemTime,              // Profile creation time
    pub last_seen: SystemTime,               // Last successful authentication
    pub total_sessions: u64,                 // Lifetime session count
    pub demotion_count: u32,                 // Lifetime demotions
    pub vouched_by: Option<String>,          // Profile ID of voucher (if any)
    pub vouched_users: Vec<String>,          // Profile IDs this user has vouched for
    pub vouching_suspended_until: Option<SystemTime>,  // Vouching privilege suspension
    pub subscription_expires: Option<SystemTime>,      // None = lifetime or Squire
    pub is_suspended: bool,                  // Temporary suspension flag
    pub is_revoked: bool,                    // Permanent revocation flag
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | UUID for profile |
| `key_fingerprint` | `String` | PGP key fingerprint (only fingerprint stored, not full key) |
| `tier` | `FastPassTier` | Squire (free) or Knight (paid) |
| `created_at` | `SystemTime` | When profile was created |
| `last_seen` | `SystemTime` | Last successful PGP authentication |
| `total_sessions` | `u64` | Total sessions created via Fast-Pass |
| `demotion_count` | `u32` | Lifetime demotion count |
| `vouched_by` | `Option<String>` | Profile ID of Knight who vouched (if applicable) |
| `vouched_users` | `Vec<String>` | Profile IDs of users this profile has vouched for |
| `vouching_suspended_until` | `Option<SystemTime>` | If set, vouching is suspended until this time |
| `subscription_expires` | `Option<SystemTime>` | Knight subscription expiry (None = lifetime or Squire) |
| `is_suspended` | `bool` | True if temporarily suspended (too many demotions) |
| `is_revoked` | `bool` | True if permanently revoked |

### FastPassTier Enum (Planned)

```rust
/// Fast-Pass membership tiers
pub enum FastPassTier {
    Squire,   // Free tier
    Knight,   // Paid tier (XMR)
}
```

| Variant | Description | Starting Trust | Captcha |
|---------|-------------|----------------|---------|
| `Squire` | Free tier - PGP identity only | Verified (+1) | 1 easy captcha per session |
| `Knight` | Paid tier - XMR subscription | Trusted (+2) | Complete bypass |

### VoucherCode Struct (Planned)

```rust
/// One-time voucher code issued by Knights
pub struct VoucherCode {
    pub code: String,               // Unique code (e.g., "KNIGHT-A7B3-C9D2")
    pub issuer_profile_id: String,  // Profile ID of issuing Knight
    pub created_at: SystemTime,     // When code was generated
    pub expires_at: SystemTime,     // Expiry time (default: 7 days)
    pub used: bool,                 // Whether code has been redeemed
    pub used_by: Option<String>,    // Profile ID of redeemer (if used)
}
```

| Field | Type | Description |
|-------|------|-------------|
| `code` | `String` | Unique voucher code |
| `issuer_profile_id` | `String` | Profile ID of Knight who issued |
| `created_at` | `SystemTime` | Generation timestamp |
| `expires_at` | `SystemTime` | Expiry timestamp (7 days default) |
| `used` | `bool` | Whether code has been redeemed |
| `used_by` | `Option<String>` | Profile ID of user who redeemed |

### FastPassConfig Struct (Planned)

```rust
/// Fast-Pass system configuration
pub struct FastPassConfig {
    pub enabled: bool,
    pub squire_config: SquireConfig,
    pub knight_config: KnightConfig,
    pub xmr_config: XmrPaymentConfig,
    pub reputation_config: ReputationConfig,
}

pub struct SquireConfig {
    pub starting_tier: TrustTier,           // Default: Verified
    pub require_easy_captcha: bool,         // Default: true
    pub key_rotations_per_month: u32,       // Default: 1
    pub registration_rate_limit_hours: u32, // Default: 24
}

pub struct KnightConfig {
    pub starting_tier: TrustTier,           // Default: Trusted
    pub captcha_bypass: bool,               // Default: true
    pub demotion_threshold_multiplier: f32, // Default: 1.5 (50% more lenient)
    pub vouchers_per_month: u32,            // Default: 3
    pub voucher_expiry_days: u32,           // Default: 7
}

pub struct XmrPaymentConfig {
    pub payment_address: String,            // XMR address for payments
    pub view_key: String,                   // View key for verification
    pub monthly_price_xmr: String,          // e.g., "0.01"
    pub yearly_price_xmr: String,           // e.g., "0.10"
    pub lifetime_price_xmr: String,         // e.g., "0.50"
}

pub struct ReputationConfig {
    pub demotions_for_temp_suspension: u32, // Default: 3 (in 30 days)
    pub temp_suspension_days: u32,          // Default: 7
    pub demotions_for_downgrade: u32,       // Default: 5 (in 90 days)
    pub demotions_for_revocation: u32,      // Default: 10 (lifetime)
    pub decay_enabled: bool,                // Default: false
    pub decay_period_days: u32,             // Default: 180
}
```

### Fast-Pass Methods (Planned)

| Method | Signature | Description |
|--------|-----------|-------------|
| `FastPassProfile::new()` | `fn new(fingerprint: &str, tier: FastPassTier) -> Self` | Create new profile |
| `FastPassProfile::verify_signature()` | `fn verify_signature(&self, challenge: &str, signature: &[u8]) -> bool` | Verify PGP signature |
| `FastPassProfile::record_demotion()` | `fn record_demotion(&mut self)` | Record demotion event |
| `FastPassProfile::check_suspension()` | `fn check_suspension(&self) -> Option<Duration>` | Check if suspended, return remaining time |
| `FastPassProfile::upgrade_to_knight()` | `fn upgrade_to_knight(&mut self, expires: SystemTime)` | Upgrade Squire to Knight |
| `FastPassProfile::downgrade_to_squire()` | `fn downgrade_to_squire(&mut self)` | Downgrade Knight to Squire |
| `FastPassProfile::revoke()` | `fn revoke(&mut self)` | Permanently revoke Fast-Pass |
| `FastPassProfile::generate_voucher()` | `fn generate_voucher(&self) -> Result<VoucherCode, Error>` | Generate voucher code (Knight only) |
| `VoucherCode::new()` | `fn new(issuer_id: &str) -> Self` | Create new voucher code |
| `VoucherCode::redeem()` | `fn redeem(&mut self, redeemer_id: &str) -> Result<(), Error>` | Redeem voucher code |
| `VoucherCode::is_valid()` | `fn is_valid(&self) -> bool` | Check if code is valid (not used, not expired) |

### Fast-Pass Configuration (Planned)

```toml
[fast_pass]
enabled = false                              # Enable Fast-Pass system

[fast_pass.squire]
starting_tier = "Verified"                   # Trust tier for Squires
require_easy_captcha = true                  # Require 1 easy captcha per session
key_rotations_per_month = 1                  # Max key rotations for free tier
registration_rate_limit_hours = 24           # Rate limit for new registrations

[fast_pass.knight]
starting_tier = "Trusted"                    # Trust tier for Knights
captcha_bypass = true                        # Complete captcha bypass
demotion_threshold_multiplier = 1.5          # 50% more lenient on demotions
vouchers_per_month = 3                       # Max voucher codes per month
voucher_expiry_days = 7                      # Voucher code expiry

[fast_pass.xmr]
payment_address = "4..."                     # XMR receiving address
view_key = "..."                             # View key for payment verification
monthly_price_xmr = "0.01"                   # Monthly subscription price
yearly_price_xmr = "0.10"                    # Yearly subscription price
lifetime_price_xmr = "0.50"                  # Lifetime access price

[fast_pass.reputation]
demotions_for_temp_suspension = 3            # Demotions to trigger temp suspension
temp_suspension_days = 7                     # Duration of temp suspension
demotions_for_downgrade = 5                  # Demotions for Knight→Squire downgrade
demotions_for_revocation = 10                # Lifetime demotions for permanent revoke
decay_enabled = false                        # Enable reputation decay
decay_period_days = 180                      # Days of inactivity before decay
```

### Fast-Pass Tier Comparison

| Feature | Anonymous User | Squire (Free) | Knight (Paid) |
|---------|---------------|---------------|---------------|
| **Starting Tier** | Unknown (0) | Verified (+1) | Trusted (+2) |
| **Captcha** | 1 normal | 1 easy | None |
| **Re-verify on Demotion** | CAPTCHA | PGP + 1 easy | PGP only |
| **Key Rotation** | N/A | 1/month | Unlimited |
| **Vouching** | No | No | Yes (3/month) |
| **Demotion Threshold** | Standard | Standard | 1.5x (lenient) |
| **Priority Routing** | No | No | Yes (if implemented) |
| **Cost** | Free | Free | XMR subscription |

---

## Summary Statistics

| Category | Count |
|----------|-------|
| **Total Structs** | 45+ |
| **Total Enums** | 15+ |
| **Total Functions** | 200+ |
| **Config Options** | 50+ |
| **API Endpoints** | 20+ |
| **Captcha Types** | 7 |
| **Trust Tiers** | 5 |
| **Violation Types** | 14 |
| **Attack Patterns** | 25+ |
| **Fast-Pass Structs (Planned)** | 6 |
| **Fast-Pass Tiers** | 2 (Squire, Knight) |

---

*Generated from Fortify source code analysis*  
*Last Updated: 2024*
