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

### GET /

Returns the gate page with CAPTCHA challenge.

**Response:**
- Content-Type: `text/html`
- Body: HTML page with CAPTCHA form

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `redirect` | string | URL to redirect after verification |
| `demoted` | bool | If true, show threat captcha |

---

### GET /captcha

Returns a new CAPTCHA image.

**Response:**
```
Content-Type: image/bmp (or image/png depending on type)
X-Captcha-Id: <challenge_id>
X-Captcha-Type: <captcha_type>
```

**Headers:**
| Header | Value |
|--------|-------|
| `X-Captcha-Id` | UUID of the challenge |
| `X-Captcha-Type` | BmpText, Emoji, Direction, etc. |

---

### POST /verify

Submit CAPTCHA answer.

**Request:**
```
Content-Type: application/x-www-form-urlencoded

captcha_id=<id>&answer=<answer>&redirect=<url>
```

**Form Fields:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `captcha_id` | string | Yes | Challenge ID |
| `answer` | string | Yes | User's answer |
| `redirect` | string | No | Redirect URL |

**Response (Success):**
```
HTTP/1.1 302 Found
Location: <redirect_url>
Set-Cookie: fortify_session=<token>; Path=/; HttpOnly; SameSite=Strict
```

**Response (Failure):**
```
HTTP/1.1 200 OK
Content-Type: text/html

<!-- Gate page with error message -->
```

---

### GET /pow

Get Proof-of-Work challenge (if enabled).

**Response:**
```json
{
  "challenge_id": "uuid",
  "prefix": "0000abc123",
  "difficulty": 20,
  "algorithm": "sha256"
}
```

**Fields:**
| Field | Type | Description |
|-------|------|-------------|
| `challenge_id` | string | UUID of challenge |
| `prefix` | string | String to prepend |
| `difficulty` | int | Number of leading zeros |
| `algorithm` | string | Hash algorithm |

---

### POST /pow/verify

Submit Proof-of-Work solution.

**Request:**
```json
{
  "challenge_id": "uuid",
  "nonce": "12345678"
}
```

**Response (Success):**
```json
{
  "success": true,
  "session_token": "base64-token"
}
```

**Response (Failure):**
```json
{
  "success": false,
  "error": "Invalid solution"
}
```

---

### GET /health

Health check endpoint.

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
