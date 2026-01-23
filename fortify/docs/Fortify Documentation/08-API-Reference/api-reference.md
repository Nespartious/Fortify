# 📡 API Reference

> **Internal and External API Documentation**

---

## API Categories

| Category | Purpose |
|----------|---------|
| **Gate API** | User verification endpoints |
| **Proxy API** | Request handling, health |
| **Admin API** | Management interface |
| **Internal API** | Component communication |

---

## Gate API

**Base URL:** `http://[gate_address]:[port]/`

### GET /Fortify

Returns the gate landing page. Includes cookie compliance check to filter bots.

**Flow:**
1. First visit → Sets `fortify_test=1` cookie, redirects to `/Fortify?check=1`
2. Second visit with cookie → Shows landing page
3. Second visit without cookie → Blocks as bot (no cookie support)

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `check` | bool | Internal parameter for cookie compliance check |

**Response:**
- Content-Type: `text/html`
- Body: Landing page HTML (gate.html or demoted.html for demoted users)

**Cookies Read:**
| Cookie | Description |
|--------|-------------|
| `fortify_test` | Cookie compliance test |
| `fortify_demoted` | Set by nodes when user is demoted |
| `fortify_original_session` | Preserved session ID during demotion |
| `fortify_pending_session` | Session ID assigned before verification |

---

### GET /Fortify/Portcullis

Returns the CAPTCHA challenge page.

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `reason` | string | Why user is seeing CAPTCHA (`rate_limit`, `demotion`, etc.) |

**Response:**
- Content-Type: `text/html`
- Body: CAPTCHA challenge page with form

**Cookies Preserved:**
- Session IDs maintained through verification process

---

### GET /gate/captcha/{id}

Returns a CAPTCHA image for the given challenge ID.

**Path Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | Challenge UUID |

**Response:**
```
Content-Type: image/bmp (or image/png depending on type)
```

**Example:**
```
GET /gate/captcha/550e8400-e29b-41d4-a716-446655440000.png
```

---

### POST /gate/verify

Submit CAPTCHA answer and receive verification token.

**Request:**
```
Content-Type: application/x-www-form-urlencoded

captcha_id=<id>&answer=<answer>
```

**Form Fields:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `captcha_id` | string | Yes | Challenge ID |
| `answer` | string | Yes | User's answer |

**Response (Success):**
```
HTTP/1.1 302 Found
Location: /
Set-Cookie: fortify_verification=<token>; Path=/; HttpOnly; SameSite=Strict; Max-Age=60
```

**Response (Failure):**
```
HTTP/1.1 200 OK
Content-Type: text/html

<!-- CAPTCHA page with error message -->
```

**Notes:**
- Verification token is **single-use** and expires in 60 seconds
- Token must be upgraded to session token on first request to site
- User-Agent binding prevents token sharing

---

### POST /gate/upgrade-token

**Internal API:** Upgrade verification token to session token.

Called by fortify-http proxy when user presents verification token.

**Request:**
```json
{
  "verification_token": "base64-encoded-token",
  "user_agent": "TorBrowser/13.0"
}
```

**Response (Success):**
```json
{
  "success": true,
  "session_token": "base64-encoded-session-token",
  "session_id": "uuid"
}
```

**Response (Failure):**
```json
{
  "success": false,
  "error": "Token expired" | "Token already used" | "User-Agent mismatch"
}
```

**Notes:**
- Atomic operation prevents token reuse
- User-Agent must match token binding
- Session token has 24-hour lifetime

---

### POST /gate/admin/captcha-config

**Admin API:** Update CAPTCHA configuration.

**Request:**
```json
{
  "gate_captcha_type": "BmpText",
  "threat_captcha_type": "Emoji",
  "random_cycling": false
}
```

**Response:**
```json
{
  "success": true,
  "config": { /* updated configuration */ }
}
```

---

### Proof-of-Work at Tor Layer (ENABLED)

**Note:** PoW (Proof-of-Work) defense is **ENABLED** at the Tor hidden service layer, not at the application HTTP layer. 

**Implementation Strategy:**
1. **Primary:** Attempts to create mirrors with `ADD_ONION` + `PoWDefensesEnabled` flag (Tor 0.4.9.2+)
2. **Fallback:** Creates file-based hidden services with `HiddenServicePoWDefensesEnabled 1` in torrc (Tor 0.4.8+)

**What PoW Protects:**
- ✅ Introduction point flooding attacks (Tor connection layer)
- ✅ Circuit creation DoS attempts
- ✅ Bot connections at Tor protocol level

**What PoW Does NOT Protect:**
- ❌ Application-layer attacks (slow-loris, malformed requests)
- ❌ Attacks from clients who already solved PoW puzzle

**Application Layer:** Uses CAPTCHA verification for human validation after PoW defense.

**No HTTP Endpoints:** PoW is transparent to application - handled entirely by Tor daemon. The following endpoints do NOT exist:

- ~~`GET /pow`~~ (not implemented - PoW at Tor layer)
- ~~`POST /pow/verify`~~ (not implemented - PoW at Tor layer)

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_seconds": 3600
}
```

---

## Proxy API

**Base URL:** `http://[proxy_address]:[port]/`

### GET /health

Health check endpoint.

**Response:**
```json
{
  "status": "ok",
  "active_sessions": 127,
  "healthy_nodes": 10,
  "threat_nodes": 3
}
```

---

### ALL /*

Proxy all requests to backend.

**Request Headers Forwarded:**
| Header | Handling |
|--------|----------|
| `Host` | Preserved |
| `Cookie` | Session token extracted |
| `X-Forwarded-For` | Added/updated |
| `X-Real-IP` | Added |

**Response Headers Added:**
| Header | Value |
|--------|-------|
| `X-Fortify-Session` | Session ID (debug mode) |
| `X-Fortify-Tier` | Trust tier (debug mode) |

---

## Admin API

**Base URL:** `http://[proxy_address]:[port]/ctrl_8f7k3m9x2n4p1q6w5v0b8c/api/`

### Session Endpoints

#### GET /sessions

List all sessions.

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `tier` | string | Filter by tier |
| `limit` | int | Max results (default: 100) |
| `offset` | int | Pagination offset |
| `sort` | string | Sort field |

**Response:**
```json
{
  "sessions": [
    {
      "id": "uuid",
      "tier": "Verified",
      "request_count": 247,
      "violation_count": 0,
      "demotion_count": 0,
      "created_at": "2024-01-15T14:32:00Z",
      "last_activity": "2024-01-15T15:30:00Z",
      "is_demoted": false,
      "is_banned": false,
      "is_killed": false,
      "current_node": "healthy-0"
    }
  ],
  "total": 127,
  "limit": 100,
  "offset": 0
}
```

---

#### GET /sessions/{id}

Get session details.

**Response:**
```json
{
  "session": {
    "id": "uuid",
    "tier": "Verified",
    "request_count": 247,
    "violation_count": 0,
    "demotion_count": 0,
    "created_at": "2024-01-15T14:32:00Z",
    "last_activity": "2024-01-15T15:30:00Z",
    "is_demoted": false,
    "is_banned": false,
    "is_killed": false,
    "current_node": "healthy-0",
    "admin_override": null
  },
  "behavioral": {
    "unique_paths": 45,
    "form_submissions": 12,
    "total_payload_bytes": 2400000,
    "severity_score": 0,
    "bot_detected": false
  },
  "violations": [
    {
      "type": "PathEnumeration",
      "count": 0,
      "severity": 0
    }
  ],
  "history": [
    {
      "type": "PageRequest",
      "timestamp": "2024-01-15T14:32:05Z",
      "details": {
        "path": "/index.html",
        "status": 200
      }
    }
  ]
}
```

---

#### POST /sessions/{id}/action

Perform action on session.

**Request:**
```json
{
  "action": "change_tier",
  "params": {
    "tier": "Suspicious"
  }
}
```

**Actions:**
| Action | Params | Description |
|--------|--------|-------------|
| `change_tier` | `tier` | Admin override tier |
| `clear_override` | - | Remove admin override |
| `ban` | - | Ban session |
| `unban` | - | Unban session |
| `kill` | - | Kill session permanently |

**Response:**
```json
{
  "success": true,
  "session": { /* updated session object */ }
}
```

---

### Node Endpoints

#### GET /nodes

List all nodes.

**Response:**
```json
{
  "healthy_nodes": [
    {
      "id": "healthy-0",
      "address": "127.0.0.1:9100",
      "status": "online",
      "active_connections": 23,
      "total_requests": 12456
    }
  ],
  "threat_nodes": [
    {
      "id": "threat-0",
      "address": "127.0.0.1:9110",
      "status": "online",
      "active_connections": 5,
      "total_violations": 156
    }
  ]
}
```

---

### Mirror Endpoints

#### GET /mirrors

List all mirrors.

**Response:**
```json
{
  "active_mirrors": [
    {
      "id": "mirror-1",
      "onion_address": "abc123...onion",
      "state": "Active",
      "created_at": "2024-01-15T12:00:00Z",
      "age_hours": 2.5,
      "compromise_score": 0.12,
      "metrics": {
        "total_requests": 45678,
        "bandwidth_bytes": 1200000000,
        "violations_detected": 156
      }
    }
  ],
  "standby_mirrors": [
    {
      "id": "mirror-2",
      "onion_address": "def456...onion",
      "state": "Standby",
      "created_at": "2024-01-15T10:00:00Z"
    }
  ],
  "burned_mirrors": [
    {
      "id": "mirror-0",
      "onion_address": "old789...onion",
      "state": "Burned",
      "burned_at": "2024-01-15T08:00:00Z",
      "burn_reason": "age_rotation"
    }
  ]
}
```

---

#### POST /mirrors/{id}/action

Perform action on mirror.

**Request:**
```json
{
  "action": "burn",
  "params": {
    "reason": "manual"
  }
}
```

**Actions:**
| Action | Params | Description |
|--------|--------|-------------|
| `activate` | - | Activate standby mirror |
| `pause` | - | Pause active mirror |
| `resume` | - | Resume paused mirror |
| `burn` | `reason` | Initiate burn sequence |

---

#### POST /mirrors/create

Create new standby mirror.

**Response:**
```json
{
  "success": true,
  "mirror": {
    "id": "mirror-3",
    "onion_address": "new123...onion",
    "state": "Standby"
  }
}
```

---

### Configuration Endpoints

#### GET /behavioral/config

Get behavioral analysis configuration.

**Response:**
```json
{
  "config": {
    "enable_user_agent_analysis": true,
    "enable_referer_analysis": true,
    "enable_path_analysis": true,
    "enable_enumeration_detection": true,
    "enable_form_tracking": true,
    "enable_payload_analysis": true,
    "max_unique_paths_per_minute": 60,
    "max_form_submissions_per_minute": 10,
    "max_payload_size_bytes": 10485760,
    "enumeration_detection_threshold": 5,
    "demotion_threshold": 10,
    "severity_demotion_threshold": 15,
    "kill_after_demotions": 3,
    "whitelisted_paths": ["/api/*", "/static/*"],
    "attack_patterns": ["../", "/.env", "/.git"]
  }
}
```

---

#### POST /behavioral/config

Update behavioral configuration.

**Request:**
```json
{
  "config": {
    "demotion_threshold": 5,
    "max_unique_paths_per_minute": 30
  }
}
```

---

#### GET /branding/config

Get branding configuration.

**Response:**
```json
{
  "config": {
    "service_name": "Fortify",
    "description": "Secure Access Gateway",
    "welcome_message": "Welcome to our service",
    "primary_color": "#c9a227",
    "secondary_color": "#a68b5b",
    "tertiary_color": "#2D3748",
    "custom_css": null
  }
}
```

---

#### POST /settings/branding

Update branding configuration via form POST.

**Request:**
```
Content-Type: application/x-www-form-urlencoded

service_name=Fortify&description=Secure+Access&primary_color=%23c9a227...
```

**Form Fields:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `service_name` | string | No | Display name for the service |
| `description` | string | No | Short service description |
| `welcome_message` | string | No | Welcome message on landing page |
| `primary_color` | string | No | Primary brand color (hex, e.g. #c9a227) |
| `secondary_color` | string | No | Secondary brand color (hex) |
| `tertiary_color` | string | No | Tertiary brand color (hex) |
| `custom_css` | string | No | Custom CSS to inject into templates |

**Response:**
```
HTTP/1.1 302 Found
Location: /ctrl_8f7k3m9x2n4p1q6w5v0b8c/settings
```

---

#### GET /captcha/config

Get CAPTCHA configuration.

**Response:**
```json
{
  "config": {
    "captcha_type": "BmpText",
    "threat_captcha_type": "Emoji",
    "enable_pow": false,
    "pow_difficulty": 20,
    "enable_cycling": false,
    "cycling_types": ["BmpText", "Emoji", "Direction"]
  }
}
```

---

#### GET /captcha-pool/config

Get CAPTCHA pool configuration.

**Response:**
```json
{
  "config": {
    "pool_size": 500,
    "min_pool_size": 100,
    "max_pool_size": 1000,
    "difficulty": 5,
    "timeout_seconds": 120,
    "max_attempts": 3,
    "rotation_percent": 25,
    "rotation_interval_days": 10
  }
}
```

---

#### POST /settings/captcha-pool

Update CAPTCHA pool configuration via form POST.

**Request:**
```
Content-Type: application/x-www-form-urlencoded

pool_size=500&min_pool_size=100&max_pool_size=1000&difficulty=5...
```

**Form Fields:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `pool_size` | int | No | Target pool size (default: 500) |
| `min_pool_size` | int | No | Minimum pool before emergency generation (default: 100) |
| `max_pool_size` | int | No | Maximum pool size (default: 1000) |
| `difficulty` | int | No | CAPTCHA difficulty 1-10 (default: 5) |
| `timeout_seconds` | int | No | Time limit to solve (default: 120) |
| `max_attempts` | int | No | Maximum solve attempts (default: 3) |
| `rotation_percent` | int | No | Pool refresh percentage (0-100) |
| `rotation_interval_days` | int | No | Rotation interval in days |

**Response:**
```
HTTP/1.1 302 Found
Location: /ctrl_8f7k3m9x2n4p1q6w5v0b8c/settings
```

---

#### POST /settings/captcha-type

Update per-type CAPTCHA configuration via form POST.

**Request:**
```
Content-Type: application/x-www-form-urlencoded

type_name=Emoji&enabled=1&option_count=6&difficulty=2&min_pool_size=50
```

**Form Fields:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type_name` | string | Yes | CAPTCHA type (BmpText, Emoji, Direction, Sequence, WordUnscramble, ImageRotation, Silhouette) |
| `enabled` | bool | No | Whether this type is enabled (checkbox) |
| `option_count` | int | No | Number of options for selection-based CAPTCHAs |
| `difficulty` | int | No | Difficulty level 1-3 |
| `min_pool_size` | int | No | Minimum pool size for this type |

**Response:**
```
HTTP/1.1 302 Found
Location: /ctrl_8f7k3m9x2n4p1q6w5v0b8c/settings
```

---

#### POST /config/save

Save current admin configuration to disk.

**Request:** No body required.

**Response:**
```
HTTP/1.1 302 Found
Location: /ctrl_8f7k3m9x2n4p1q6w5v0b8c/settings
```

**Notes:**
- Saves to `/etc/fortify/admin-state.json`
- Creates parent directory if needed
- Persists: branding, captcha pool, behavior config, per-type settings

---

#### POST /config/reload

Reload admin configuration from disk.

**Request:** No body required.

**Response:**
```
HTTP/1.1 302 Found
Location: /ctrl_8f7k3m9x2n4p1q6w5v0b8c/settings
```

**Notes:**
- Loads from `/etc/fortify/admin-state.json`
- Overwrites current runtime configuration
- No restart required

---

### Statistics Endpoints

#### GET /stats

Get global statistics.

**Response:**
```json
{
  "sessions": {
    "total": 127,
    "by_tier": {
      "Trusted": 12,
      "Verified": 89,
      "Unknown": 15,
      "Suspicious": 8,
      "Burned": 3
    },
    "demoted": 23,
    "banned": 5,
    "killed": 2
  },
  "violations": {
    "total_24h": 156,
    "by_type": {
      "PathEnumeration": 45,
      "AttackPathAccess": 32,
      "FormSubmissionFlood": 28,
      "MissingUserAgent": 21,
      "SuspiciousUserAgent": 15,
      "RapidRequests": 10,
      "PayloadOverflow": 5
    }
  },
  "mirrors": {
    "active": 2,
    "standby": 3,
    "burned_24h": 1
  },
  "nodes": {
    "healthy": 10,
    "threat": 3,
    "total_requests_24h": 456789
  },
  "traffic": {
    "requests_24h": 456789,
    "bandwidth_24h_bytes": 12000000000,
    "peak_rps": 1245,
    "avg_response_time_ms": 234
  }
}
```

---

## Internal API

### Node → Session Manager

```rust
// Report violation
pub async fn report_violation(
    session_id: &str,
    violation_type: ViolationType,
    severity: u32,
) -> Result<(), Error>

// Update session metrics
pub async fn update_session_metrics(
    session_id: &str,
    metrics: SessionMetrics,
) -> Result<(), Error>
```

---

### Orchestrator → Tor Service

```rust
// Create onion service
pub async fn create_onion_service(
    port: u16,
    target_port: u16,
) -> Result<String, TorError>  // Returns .onion address

// Remove onion service
pub async fn remove_onion_service(
    onion_address: &str,
) -> Result<(), TorError>

// Set PoW parameters
pub async fn set_pow_params(
    onion_address: &str,
    effort: u32,
) -> Result<(), TorError>
```

---

### Controller → Vanguards

```rust
// Start vanguards
pub async fn start_vanguards() -> Result<(), Error>

// Stop vanguards
pub async fn stop_vanguards() -> Result<(), Error>

// Get status
pub async fn get_vanguards_status() -> VanguardsStatus

// Trigger rotation
pub async fn trigger_rotation() -> Result<(), Error>
```

---

## Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `SESSION_NOT_FOUND` | 404 | Session ID doesn't exist |
| `INVALID_TOKEN` | 401 | Session token invalid/expired |
| `RATE_LIMITED` | 429 | Too many requests |
| `CAPTCHA_FAILED` | 400 | Wrong CAPTCHA answer |
| `POW_FAILED` | 400 | Invalid PoW solution |
| `BANNED` | 403 | Session is banned |
| `KILLED` | 403 | Session is killed |
| `NODE_UNAVAILABLE` | 503 | No available backend nodes |
| `MIRROR_NOT_FOUND` | 404 | Mirror ID doesn't exist |
| `INTERNAL_ERROR` | 500 | Internal server error |

---

*API responses are subject to change. Always check version compatibility.*
