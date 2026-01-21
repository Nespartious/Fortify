# Fortify Roadmap R1 - Comprehensive Development Plan

**Version:** R1 (Revision 1)  
**Date:** January 15, 2026  
**Status:** Living Document  

---

## Executive Summary

Fortify is a defensive protection layer for Tor hidden services with **19,325+ lines of Rust code** across 7 crates. The system is in **Beta** status with core functionality complete and operational.

### Project Statistics
| Metric | Value |
|--------|-------|
| Total Lines of Code | 19,325+ |
| Crates | 7 (core, gate, http, node, orchestrator, controller, community) |
| Completed Phases | 1, 2, 2.5, partial 3 |
| Security Audit Score | 68/100 |

### Architecture Overview
```
┌─────────────────────────────────────────────────────────────────┐
│                    PUBLIC INTERNET (Tor)                        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│              PUBLIC MIRRORS (Orchestrators)                      │
│         Multiple disposable .onion entry points                  │
│              Rotated proactively on schedule                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      GATE (Verification)                         │
│    • Server-side captcha generation (BMP, CSS puzzles)          │
│    • Multi-captcha for demoted users (2 required)               │
│    • Token issuance after verification                          │
│    • No JavaScript - pure HTML/CSS challenges                    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    HTTP PROXY (Fast Path)                        │
│    • Token validation (HMAC-SHA256)                              │
│    • Behavioral analysis engine                                  │
│    • Trust tier enforcement                                      │
│    • Admin control panel (3,791 lines)                          │
└─────────────────────────────────────────────────────────────────┘
                              │
                    ┌─────────┴─────────┐
                    ▼                   ▼
┌──────────────────────────┐ ┌──────────────────────────┐
│    HEALTHY NODE POOL     │ │    THREAT NODE POOL      │
│   • Verified sessions    │ │   • Suspicious sessions  │
│   • Fast forwarding      │ │   • Rate limited         │
│   • 1000 req/min         │ │   • 100 req/min          │
└──────────────────────────┘ └──────────────────────────┘
                    │                   │
                    └─────────┬─────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    PROTECTED SERVICE                             │
│              (Real .onion address - NEVER exposed)              │
└─────────────────────────────────────────────────────────────────┘
```

---

## Phase Completion Status

### ✅ PHASE 1: Foundation (COMPLETE)
- [x] Core architecture (Controller, Orchestrator, Nodes, Gate)
- [x] Trust tier system (Unknown → Suspicious → Verified → Trusted → Burned)
- [x] Session token management with HMAC-SHA256 signing
- [x] Proxy routing based on trust level
- [x] Basic violation detection
- [x] Admin control panel with real-time stats
- [x] Mirror management system
- [x] Captcha gate for verification
- [x] Friendly redirect for demoted users

### ✅ PHASE 2: Enhanced Detection (COMPLETE)
- [x] Behavioral Analysis Engine
  - Request pattern fingerprinting
  - Path traversal detection
  - User-agent anomaly detection
  - Referer chain validation
  - Per-session behavioral statistics
- [x] Content-Based Detection
  - Payload size anomaly detection
  - Form submission pattern tracking
  - Resource enumeration detection
- [x] Session Intelligence
  - Session age vs behavior analysis
  - Silent demotion/promotion

### ✅ PHASE 2.5: Node-Onion Architecture (COMPLETE)
- [x] Each node gets its own .onion address
- [x] Separate Tor daemon for healthy/threat pools
- [x] Node lifecycle management with burn logic
- [x] 24-hour grace period death page
- [x] Admin panel per-node controls

### ⏳ PHASE 3: Defensive Capabilities (PARTIAL)
- [x] Vanguards Integration (Layer 2/3 guard protection)
- [x] Progressive response delays
- [x] Multiple captcha types (BMP text, CSS puzzles, emoji, silhouette)
- [x] Multi-captcha requirement for demoted users
- [x] Dynamic rate limiting based on load
- [x] Bandwidth throttling for threat tier
- [x] Resource exhaustion traps

#### 3.1 Dynamic Rate Limiting Based on Load

**Goal:** Automatically adjust rate limits based on current system load, protecting resources during attacks while maintaining service during normal operation.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    DYNAMIC RATE LIMITING                                         │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   System Load         Rate Limit Multiplier        Effect                       │
│   ────────────────────────────────────────────────────────────────────          │
│   0-50% CPU           1.0x (normal)                Full rate limits             │
│   50-70% CPU          0.75x                        25% reduction                │
│   70-85% CPU          0.5x                         50% reduction                │
│   85-95% CPU          0.25x                        75% reduction (emergency)    │
│   95%+ CPU            0.1x                         90% reduction (survival)     │
│                                                                                  │
│   Applied per-tier:                                                             │
│   • Trusted:    1000/min × multiplier                                           │
│   • Verified:   500/min × multiplier                                            │
│   • Suspicious: 100/min × multiplier (floor: 10/min)                            │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

| Task | Status | Priority |
|------|--------|----------|
| System load monitor (CPU, memory, connections) | ⬜ Not Started | HIGH |
| Per-tier rate limit multiplier | ⬜ Not Started | HIGH |
| Graduated slowdown (not hard blocks) | ⬜ Not Started | MEDIUM |
| Burst allowance for legitimate browsing | ⬜ Not Started | MEDIUM |
| Per-path rate limiting (e.g., /api/* stricter) | ⬜ Not Started | LOW |
| Admin panel load visualization | ⬜ Not Started | LOW |

**Configuration (Planned):**
```toml
[rate_limiting.dynamic]
enabled = true
check_interval_ms = 1000
cpu_threshold_warning = 50
cpu_threshold_critical = 85
memory_threshold_warning = 70
memory_threshold_critical = 90
connection_threshold = 5000
min_rate_multiplier = 0.1      # Never go below 10% of normal
recovery_delay_seconds = 30     # Wait before increasing limits again
```

#### 3.2 Bandwidth Throttling for Threat Tier

**Goal:** Limit bandwidth consumption for suspicious/threat-tier sessions to prevent resource exhaustion attacks while still allowing limited access.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    BANDWIDTH THROTTLING BY TIER                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   Trust Tier       Bandwidth Limit      Response Delay       Notes              │
│   ────────────────────────────────────────────────────────────────────          │
│   Trusted          Unlimited            None                 Fast path          │
│   Verified         10 MB/min            None                 Normal path        │
│   Unknown          5 MB/min             None                 Pre-verification   │
│   Suspicious       1 MB/min             +500ms per request   Throttled path     │
│   Demoted          500 KB/min           +1000ms per request  Heavy throttle     │
│                                                                                  │
│   Implementation:                                                               │
│   • Token bucket algorithm per session                                          │
│   • Soft limit: slow down responses                                             │
│   • Hard limit: 429 Too Many Requests                                           │
│   • Burst allowance: 2x limit for 10 seconds                                    │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

| Task | Status | Priority |
|------|--------|----------|
| Token bucket rate limiter per session | ⬜ Not Started | HIGH |
| Per-tier bandwidth limits | ⬜ Not Started | HIGH |
| Progressive response delays | ✅ Complete | - |
| Large response chunking/streaming | ⬜ Not Started | MEDIUM |
| Bandwidth monitoring in admin panel | ⬜ Not Started | LOW |

**Configuration (Planned):**
```toml
[bandwidth_throttling]
enabled = true

[bandwidth_throttling.limits]
trusted_mb_per_min = 0           # 0 = unlimited
verified_mb_per_min = 10
unknown_mb_per_min = 5
suspicious_mb_per_min = 1
demoted_kb_per_min = 500

[bandwidth_throttling.delays]
suspicious_delay_ms = 500
demoted_delay_ms = 1000
cumulative = true                # Delays stack with each request

[bandwidth_throttling.burst]
enabled = true
multiplier = 2.0
duration_seconds = 10
```

#### 3.3 Resource Exhaustion Traps (Honeypot Endpoints)

**Goal:** Deploy honeypot endpoints that appear valuable to attackers but waste their resources while providing intelligence about attack patterns.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    HONEYPOT ENDPOINT TYPES                                       │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   Type              Endpoint Examples            Trap Behavior                  │
│   ────────────────────────────────────────────────────────────────────          │
│   Admin Honeypot    /admin, /wp-admin,          Fake login form, logs creds,   │
│                     /administrator              infinite CAPTCHA loop           │
│                                                                                  │
│   API Honeypot      /api/v1/users,              Returns fake data slowly,      │
│                     /api/admin/config            tarpits connection             │
│                                                                                  │
│   File Honeypot     /.env, /.git/config,        Returns fake secrets,          │
│                     /backup.sql                  flags session immediately      │
│                                                                                  │
│   Directory Trap    /secret/, /private/,        Infinite directory listing,    │
│                     /backup/                     each page slower than last     │
│                                                                                  │
│   Form Trap         Hidden form fields          Auto-demote if filled          │
│                     (CSS hidden from humans)    (bot detection)                │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

| Task | Status | Priority |
|------|--------|----------|
| Honeypot endpoint registry | ⬜ Not Started | MEDIUM |
| Fake admin panel trap | ⬜ Not Started | MEDIUM |
| Tarpit response generator (slow drip) | ⬜ Not Started | MEDIUM |
| Fake secrets file generator | ⬜ Not Started | LOW |
| Hidden form field bot detection | ⬜ Not Started | HIGH |
| Honeypot hit logging & analytics | ⬜ Not Started | MEDIUM |
| Immediate demotion on trap trigger | ⬜ Not Started | HIGH |

**Configuration (Planned):**
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
response = "fake_env"
immediate_burn = true           # Access = instant Burned tier

[[honeypots.endpoints]]
path = "/api/v1/users"
type = "api_trap"
response = "fake_json"
tarpit_seconds = 60
page_size = 10
infinite_pagination = true      # Always returns "next page" link

[honeypots.hidden_fields]
enabled = true
field_name = "website_url"      # Common honeypot field
css_hide = true                 # Hidden via CSS, bots fill it
fill_action = "demote"          # demote | burn | flag
```

**Tarpit Implementation:**
```
Normal Response:    [============================] 200ms
Tarpit Response:    [=.=.=.=.=.=.=.=.=.=.=.=.=.=] 30,000ms
                    (data dripped 1 byte at a time)
```

---

## CURRENT PRIORITY: Phase 4 - Resilience & Recovery

### 4.1 Mirror Rotation (In Progress)
| Task | Status | Priority |
|------|--------|----------|
| Time-based mirror rotation | ✅ Complete | - |
| Automatic mirror spawning on threat | ⬜ Not Started | HIGH |
| Graceful drain of burned mirrors | ⬜ Not Started | HIGH |
| DNS-like pointer for discovery | ⬜ Not Started | MEDIUM |
| Health scoring for mirrors | ⬜ Not Started | MEDIUM |

### 4.2 Attack Logging & Forensics
| Task | Status | Priority |
|------|--------|----------|
| Structured attack logging (no PII) | ⬜ Not Started | HIGH |
| Attack pattern database | ⬜ Not Started | MEDIUM |
| Automated incident reports | ⬜ Not Started | MEDIUM |
| Historical trend analysis | ⬜ Not Started | LOW |

### 4.3 Recovery Procedures
| Task | Status | Priority |
|------|--------|----------|
| Automatic service restart on crash | ⬜ Not Started | HIGH |
| State recovery after reboot | ⬜ Not Started | HIGH |
| Session persistence across restarts | ⬜ Not Started | MEDIUM |
| Graceful degradation modes | ⬜ Not Started | MEDIUM |

### 4.4 Multi-Daemon Architecture (NEW - HIGH PRIORITY)
**Rationale:** PoW alone has a ~10% residual risk. State-level botnets (100k+ bots) can overwhelm a single daemon just verifying PoW answers. One Tor daemon per CPU core provides complete isolation.

| Task | Status | Priority |
|------|--------|----------|
| Spawn N Tor daemons (N = CPU cores) | ⬜ Not Started | HIGH |
| Unique ports per daemon | ⬜ Not Started | HIGH |
| CPU affinity pinning (taskset) | ⬜ Not Started | HIGH |
| Per-daemon health monitoring | ⬜ Not Started | HIGH |
| Mirror distribution across daemons | ⬜ Not Started | MEDIUM |

### 4.5 Session Continuity (NEW - MEDIUM PRIORITY)

**Problem:** Users who browse via Tor on a VM often pause their VM between sessions. When they return (hours or even days later), their browser still has the old session token in cookies, but that token has expired. Currently, they're forced back through the Gate/CAPTCHA even though they were a known-good session.

**Solution:** Maintain a lightweight session history database that persists session IDs and their last-known trust status. When an expired token is presented, look up the session ID in history and restore the user to their previous status (even if assigned a new session ID).

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                         SESSION CONTINUITY FLOW                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   USER JOURNEY (VM Pause Scenario)                                              │
│   ──────────────────────────────────────────────────────────────────────────    │
│                                                                                  │
│   Day 1, 2:00 PM                                                                │
│   ┌──────────────────────┐                                                      │
│   │ User browses site    │                                                      │
│   │ Status: VERIFIED     │                                                      │
│   │ Session: abc-123     │                                                      │
│   └──────────┬───────────┘                                                      │
│              │                                                                   │
│              ▼                                                                   │
│   User pauses VM, goes to sleep                                                 │
│              │                                                                   │
│              │  ← 12 hours pass                                                 │
│              │                                                                   │
│              ▼                                                                   │
│   Day 2, 2:00 AM                                                                │
│   ┌──────────────────────┐                                                      │
│   │ User resumes VM      │                                                      │
│   │ Clicks link on page  │                                                      │
│   │ Sends: abc-123       │  ← Token expired, but session ID sent               │
│   └──────────┬───────────┘                                                      │
│              │                                                                   │
│              ▼                                                                   │
│   ┌──────────────────────────────────────────────────────────────────────────┐ │
│   │                    SESSION CONTINUITY CHECK                               │ │
│   ├──────────────────────────────────────────────────────────────────────────┤ │
│   │                                                                           │ │
│   │   1. Token expired? → YES                                                 │ │
│   │   2. Session ID in history DB? → YES (abc-123 found)                     │ │
│   │   3. History record < 7 days old? → YES (12 hours)                       │ │
│   │   4. Last status? → VERIFIED                                              │ │
│   │   5. Was killed/burned? → NO                                              │ │
│   │                                                                           │ │
│   │   ACTION: Issue new token with VERIFIED status                           │ │
│   │           New session ID: def-456                                         │ │
│   │           Link history: abc-123 → def-456                                │ │
│   │                                                                           │ │
│   └──────────────────────────────────────────────────────────────────────────┘ │
│              │                                                                   │
│              ▼                                                                   │
│   ┌──────────────────────┐                                                      │
│   │ User continues       │                                                      │
│   │ browsing seamlessly  │                                                      │
│   │ No CAPTCHA required! │                                                      │
│   └──────────────────────┘                                                      │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Key Rules:**

| Rule | Description |
|------|-------------|
| **7-Day Maximum** | Session history expires after 7 days - no indefinite persistence |
| **Status Transfer** | New session inherits last-known status (Verified, Suspicious, etc.) |
| **NOT Immunity** | Restored sessions are still subject to behavioral analysis |
| **Killed = Killed** | If previous session was killed, continuity is DENIED |
| **Burned = Burned** | If previous session was burned, continuity is DENIED |
| **New Session ID** | Always issue NEW session ID, just transfer status |
| **Demotion Count Transfers** | If they had 2 demotions, new session starts with 2 |
| **Violation Count Resets** | Start fresh violation count (benefit of the doubt) |

**What Gets Stored (Minimal - Privacy Conscious):**

```rust
pub struct SessionHistoryRecord {
    pub session_id: String,           // UUID
    pub last_trust_tier: TrustTier,   // Last known status
    pub demotion_count: u32,          // Carries over
    pub was_killed: bool,             // Permanent flag
    pub was_burned: bool,             // Permanent flag
    pub created_at: u64,              // Original session creation
    pub last_seen_at: u64,            // Last activity timestamp
    pub expires_at: u64,              // 7 days from last_seen
    pub successor_id: Option<String>, // If continued, link to new session
}
```

**NOT Stored (Privacy):**
- ❌ IP addresses
- ❌ Request paths/URLs
- ❌ User agents
- ❌ Any behavioral data
- ❌ Violation details

| Task | Status | Priority |
|------|--------|----------|
| Session history database (SQLite/sled) | ⬜ Not Started | HIGH |
| History record creation on session start | ⬜ Not Started | HIGH |
| History lookup on expired token | ⬜ Not Started | HIGH |
| Status transfer to new session | ⬜ Not Started | HIGH |
| 7-day expiry enforcement | ⬜ Not Started | MEDIUM |
| Killed/Burned denial logic | ⬜ Not Started | HIGH |
| Session linking (old → new) | ⬜ Not Started | MEDIUM |
| Admin panel history view | ⬜ Not Started | LOW |
| History cleanup job (daily) | ⬜ Not Started | MEDIUM |

**Configuration (Planned):**
```toml
[session_continuity]
enabled = true
max_age_days = 7                    # Maximum history retention
storage_backend = "sqlite"          # sqlite | sled | memory
database_path = "/var/lib/fortify/sessions.db"

[session_continuity.transfer]
transfer_tier = true                # Transfer trust tier
transfer_demotion_count = true      # Transfer demotion count  
reset_violation_count = true        # Fresh start on violations
deny_if_killed = true               # Block killed sessions
deny_if_burned = true               # Block burned sessions

[session_continuity.cleanup]
run_interval_hours = 24             # How often to clean expired
vacuum_on_cleanup = true            # SQLite vacuum after cleanup
```

**Edge Cases:**

| Scenario | Behavior |
|----------|----------|
| Token expired, session found, status=Verified | ✅ Issue new Verified session |
| Token expired, session found, status=Suspicious | ✅ Issue new Suspicious session (must re-verify) |
| Token expired, session found, was_killed=true | ❌ Deny continuity, treat as Unknown |
| Token expired, session NOT found | ❌ Normal flow, redirect to Gate |
| Token expired, session found, >7 days old | ❌ History expired, redirect to Gate |
| Token valid (not expired) | ✅ Normal flow, no history lookup needed |
| Session in history continued 3+ times | ⚠️ Consider flagging for review |

**Security Considerations:**

1. **No Indefinite Trust**: 7-day limit prevents session tokens from becoming permanent credentials
2. **Demotion Memory**: Bad actors can't "reset" by pausing - demotion count follows them
3. **Kill/Burn Permanent**: Serious offenders can't return via continuity
4. **Minimal Storage**: Only session ID + status, no tracking data
5. **New ID Always**: Never reuse session IDs, preventing replay attacks

---

## PHASE 5: Fortify Cluster System (NEW)

### Vision
Connect multiple physically separated Fortify VPS instances to protect the same service. Each VPS contributes mirrors to a unified entry point, shares threat intelligence, and provides mutual failover.

### 5.1 Multi-VPS Federation

```
┌─────────────────┐    WireGuard    ┌─────────────────┐
│   VPS Alpha     │◄──────────────►│   VPS Beta      │
│                 │    Encrypted    │                 │
│ • Mirror A1     │    Tunnel       │ • Mirror B1     │
│ • Mirror A2     │                 │ • Mirror B2     │
│ • Gate          │                 │ • Gate          │
│ • Nodes         │                 │ • Nodes         │
└─────────────────┘                 └─────────────────┘
        │                                   │
        └───────────────┬───────────────────┘
                        │
                        ▼
              ┌─────────────────┐
              │ Shared State:   │
              │ • Sessions      │
              │ • Threat Intel  │
              │ • Mirror Health │
              └─────────────────┘
```

| Task | Status | Effort |
|------|--------|--------|
| Secure inter-cluster WireGuard tunnels | ⬜ Not Started | 2 days |
| Shared session state sync | ⬜ Not Started | 3 days |
| Distributed threat intel sharing | ⬜ Not Started | 2 days |
| Computational load sharing (PoW) | ⬜ Not Started | 3 days |
| Mirror distribution (1+ per VPS) | ⬜ Not Started | 1 day |
| Automatic cluster failover | ⬜ Not Started | 3 days |
| Cluster heartbeat system | ⬜ Not Started | 1 day |

### 5.2 Public Mirror Discovery Bar
A slim header bar displayed on Gate/intro pages showing all available mirrors across the cluster.

```html
┌─────────────────────────────────────────────────────────────────┐
│ Available Mirrors: [●Alpha-1] [●Alpha-2] [○Beta-1] [●Beta-2]   │
│                     (● = healthy, ○ = degraded)                 │
└─────────────────────────────────────────────────────────────────┘
```

| Task | Status | Effort |
|------|--------|--------|
| Header bar component | ⬜ Not Started | 1 day |
| Real-time mirror health indicators | ⬜ Not Started | 1 day |
| User-selectable entry points | ⬜ Not Started | 1 day |
| Theme-aware styling | ⬜ Not Started | 0.5 days |

### 5.3 Cluster Configuration Schema
```toml
[cluster]
enabled = true
mode = "member"                    # "primary" | "member"
cluster_name = "my-service-cluster"
node_id = "alpha"                  # Unique per VPS

[cluster.wireguard]
interface = "wg-fortify"
listen_port = 51820
private_key_path = "/etc/fortify/cluster-wg.key"

[cluster.peers]
[[cluster.peers.peer]]
name = "beta"
public_key = "..."
endpoint = "10.0.0.2:51820"
allowed_ips = "10.0.1.0/24"

[cluster.sync]
interval_ms = 1000
session_sync = true
threat_intel_sync = true
mirror_health_sync = true
```

---

## PHASE 6: Deployment Wizard (NEW)

### Vision
Replace manual configuration with a guided UI-based deployment experience. The wizard walks operators through initial setup, personalization, and cluster joining.

### 6.1 Deployment Interface
| Option | Description |
|--------|-------------|
| Terminal UI (TUI) | Interactive terminal interface using `ratatui` |
| Local Web UI | Lightweight web interface on localhost only |

### 6.2 Deployment Modes

```
┌─────────────────────────────────────────────────────────────────┐
│                    FORTIFY DEPLOYMENT WIZARD                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Select deployment mode:                                         │
│                                                                  │
│  [ ] New Deployment                                              │
│      Fresh install with full configuration                       │
│                                                                  │
│  [ ] Wipe & Reinstall                                           │
│      Clean slate, preserves identity keys                        │
│                                                                  │
│  [ ] Join Cluster                                                │
│      Connect to existing Fortify cluster                         │
│                                                                  │
│  [ ] Upgrade                                                     │
│      Migrate from previous version                               │
│                                                                  │
│                                         [Next →]                 │
└─────────────────────────────────────────────────────────────────┘
```

### 6.3 Branding & Customization

The wizard allows operators to personalize their Fortify instance, shrinking Fortify branding in favor of their own.

| Setting | Description | Default |
|---------|-------------|---------|
| Site Name | Name displayed on Gate pages | "Fortify" |
| Theme | Light or Dark theme | Dark |
| Custom Logo | Logo file for Gate pages | Fortify logo |
| Primary Color | Main accent color | #05d9e8 (cyan) |
| Badge Visibility | Show/hide Fortify badge | Visible |

**Example: Custom Branding**
```
Before (Default):
┌─────────────────────────┐
│      🛡️ FORTIFY         │
│   Verification Gate     │
└─────────────────────────┘

After (Custom):
┌─────────────────────────┐
│    🌐 MyService         │
│   Security Checkpoint   │
│         [Fortify]       │  ← Small badge
└─────────────────────────┘
```

### 6.4 Vanity Address Generation
| Setting | Description |
|---------|-------------|
| Enable Vanity | Generate custom .onion prefixes |
| Prefix Characters | e.g., "mysite" → mysiteXXX...XXX.onion |
| Background Gen | Generate while wizard continues |
| Multiple Vanity | Different prefixes per mirror |

### 6.5 Network Configuration
| Setting | Default | Notes |
|---------|---------|-------|
| HTTP Proxy Port | 8082 | Main traffic entry |
| Gate Port | 8081 | Verification endpoint |
| Orchestrator Port | 8080 | Internal coordination |
| Controller Port | 7000 | Management API |
| Tor SOCKS Port | 9050 | Tor daemon |
| Tor Control Port | 9051 | Tor control |
| WireGuard Port | 51820 | Cluster communication |

### 6.6 Database & Backup Configuration
| Setting | Description |
|---------|-------------|
| Backup Destination | Encrypted off-site location |
| Sync Interval | How often to sync state |
| Snapshot Schedule | Cron-style scheduling |
| Recovery Key | Master key for backup decryption |

### 6.7 Security Configuration
| Setting | Default | Range |
|---------|---------|-------|
| Rate Limit (Verified) | 1000/min | 100-5000 |
| Rate Limit (Threat) | 100/min | 10-500 |
| Captcha Difficulty | Medium | Easy/Medium/Hard |
| Token Lifetime | 1 hour | 15min - 24hr |
| Multi-Captcha (Demoted) | 2 | 1-5 |

### 6.8 Secrets Protection (CRITICAL)

**Threat Model:** Physical seizure of VPS, RAM forensics, disk imaging.

| Protection | Implementation |
|------------|----------------|
| Zero Cleartext | All secrets encrypted at rest with AES-256-GCM |
| Memory Protection | Secrets in mlock'd memory, zeroed when not in use |
| TPM Integration | Use TPM/secure enclave where available |
| Key Derivation | Master password → Argon2id → encryption keys |
| Secure Wipe | Panic command zeros all secrets immediately |

**Minimum Protection Guarantees:**
- ✅ Real .onion address of protected service (MUST be encrypted)
- ✅ IP addresses of cluster peers (MUST be encrypted)
- ✅ Session signing keys (MUST be encrypted)
- ✅ WireGuard private keys (MUST be encrypted)

---

## PHASE 7: Community Network

### 7.1 Federated Threat Intelligence
| Task | Status |
|------|--------|
| Anonymous threat signature sharing | ⬜ Not Started |
| Community blacklist federation | ⬜ Not Started |
| Reputation exchange protocol | ⬜ Not Started |
| Attack pattern propagation | ⬜ Not Started |

### 7.2 Discovery Network
| Task | Status |
|------|--------|
| Decentralized orchestrator discovery | ⬜ Not Started |
| Mirror advertisement system | ⬜ Not Started |
| Load sharing across community | ⬜ Not Started |
| Trust-based peering | ⬜ Not Started |

---

## PHASE 8: Advanced Capabilities

### 8.1 Machine Learning Detection (Optional)
- Local-only anomaly detection
- Privacy-preserving pattern matching
- No data export to third parties

### 8.2 Integration Points
| Integration | Purpose |
|-------------|---------|
| Webhook Alerts | Real-time attack notifications |
| Prometheus/Grafana | Metrics visualization |
| Syslog | Centralized logging |
| External Blocklists | Import community blocklists |

### 8.3 Operational Tools
| Tool | Purpose |
|------|---------|
| CLI Interface | `fortifyctl` command-line management |
| Hot Reload | Config changes without restart |
| Rolling Updates | Zero-downtime upgrades |
| Backup/Restore | State preservation |

---

## Security Audit Priorities

Based on [Roadmap V2 - Security Audit.md](Roadmap%20V2%20-%20Security%20Audit.md), the following are prioritized:

### Tier 1: Critical (Implement Immediately)
1. ✅ **Vanguards Integration** - COMPLETE
2. ⬜ **OnionBalance Integration** - Survives single-backend DoS
3. ⬜ **Multi-Daemon Architecture** - CPU isolation per core
4. ⬜ **Automated Leak Scanner** - Detect configuration leaks

### Tier 2: High Priority
5. ⬜ **Fail-Closed Guard Rotation** - Prevents Sniper Attack
6. ⬜ **ASN-Diverse Guard Selection** - Prevents Sybil positioning
7. ⬜ **Cover Traffic Generation** - Defeats bandwidth correlation
8. ⬜ **Response Size Normalization** - Defeats fingerprinting

### Tier 3: Medium Priority
9. ⬜ **Tor Version Enforcement** - Ensures patched vulnerabilities
10. ⬜ **Guard Reputation Tracking** - Long-term Sybil defense
11. ⬜ **Circuit Padding Framework** - Deep fingerprinting defense

---

## Configuration Reference

### Current Config Structure (fortify.example.toml)
```toml
[service]
real_onion_address = "http://xxx.onion"
real_service_port = 80

[controller]
bind_address = "127.0.0.1:7000"
max_orchestrators = 5
max_healthy_nodes = 10
max_threat_nodes = 5

[orchestrator]
bind_address = "127.0.0.1:8080"
rotation_interval_hours = 24

[gate]
bind_address = "127.0.0.1:8081"
max_concurrent_verifications = 10
verification_timeout_seconds = 300
captcha_difficulty = "medium"

[http_proxy]
bind_address = "127.0.0.1:8082"
max_concurrent_connections = 1000

[node]
bind_base = "127.0.0.1:9100"

[community]
enabled = false
mode = "standalone"

[logging]
level = "info"
output = "syslog"
```

### Planned Config Additions

```toml
# Multi-Daemon Architecture
[tor.multi_daemon]
enabled = true
daemons_per_vps = 4
cpu_affinity = true

# Cluster Configuration
[cluster]
enabled = true
mode = "member"
cluster_name = "my-service-cluster"
wireguard_interface = "wg-fortify"

# Branding Configuration
[branding]
site_name = "MyService"
theme = "dark"
logo_path = "/etc/fortify/logo.png"
primary_color = "#05d9e8"
show_fortify_badge = true

# Secrets Protection
[security.secrets]
encryption_enabled = true
key_derivation = "argon2id"
tpm_enabled = false
secure_wipe_enabled = true

# Vanity Generation
[mirrors.vanity]
enabled = false
prefix = "mysite"
```

---

## Development Priorities (Q1 2026)

### January 2026
- [ ] Multi-Daemon Architecture implementation
- [ ] OnionBalance research and design
- [ ] Automated leak scanner MVP

### February 2026
- [ ] Cluster System Phase 1 (WireGuard, state sync)
- [ ] Deployment Wizard TUI MVP
- [ ] Fail-closed guard rotation

### March 2026
- [ ] Cluster System Phase 2 (health monitoring, failover)
- [ ] Branding customization system
- [ ] Vanity address generation

---

## Non-Goals (Explicitly Out of Scope)

- ❌ Client-side JavaScript (ever)
- ❌ Offensive capabilities
- ❌ User tracking beyond sessions
- ❌ Data export to third parties
- ❌ Breaking Tor anonymity
- ❌ Storing PII

---

## Document History

| Version | Date | Changes |
|---------|------|---------|
| R1 | 2026-01-15 | Initial comprehensive roadmap |

---

## Related Documentation

- [Architecture](architecture.md) - System design details
- [Trust Levels](trust-levels.md) - Trust tier documentation
- [Threat Model](threat-model.md) - Security threat analysis
- [Scaling Model](scaling-model.md) - Capacity planning
- [Security Audit](Roadmap%20V2%20-%20Security%20Audit.md) - Detailed security review
- [Hardening](hardening.md) - OS hardening guide
- [Community Network](community-network.md) - Federation design
