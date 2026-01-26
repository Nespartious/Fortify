# Fortify Development Roadmap

## Current Status: Beta

Fortify is a defensive protection layer for Tor hidden services. The core architecture is fully functional with session management, trust tiers, proxy routing, behavioral analysis, multi-captcha system, and node-onion architecture for circuit isolation.

---

## Phase 1: Foundation (Complete ✅)

- [x] Core architecture (Controller, Orchestrator, Nodes, Gate)
- [x] Trust tier system (Unknown → Suspicious → Verified)
- [x] Session token management with HMAC signing
- [x] Proxy routing based on trust level
- [x] Basic violation detection (rate limiting, request patterns)
- [x] Admin control panel with real-time stats
- [x] Mirror management system
- [x] Captcha gate for verification
- [x] Friendly redirect for demoted users

---

## Phase 2: Enhanced Detection (Complete ✅)

### 2.1 Behavioral Analysis Engine
- [x] Request pattern fingerprinting
- [x] Path traversal detection
- [x] User-agent anomaly detection (flags non-Tor/bot UAs)
- [x] Referer chain validation (missing=normal for Tor, suspicious presence=flag)
- [x] Admin panel toggles for each detection module
- [x] Per-session behavioral statistics in admin panel

### 2.2 Content-Based Detection
- [x] Payload size anomaly detection
- [x] Form submission patterns (rate tracking)
- [x] Resource enumeration detection (rapid unique path scanning)

### 2.3 Session Intelligence
- [x] Session age vs behavior analysis (within-session only)

**Note:** The following were explicitly skipped as not appropriate for Tor:
- ❌ Inter-request timing analysis (Tor adds natural jitter)
- ❌ Cross-session correlation (violates Tor privacy model)
- ❌ Trust velocity tracking (ephemeral sessions in Tor make this meaningless)

---

## Phase 2.5: Node-Onion Architecture (Complete ✅)

**Goal:** Create disposable Tor circuits per node, enabling true circuit-level isolation when burning compromised nodes.

### Architecture Overview
```
PUBLIC MIRRORS (permanent, rarely burn)
         │
    FORTIFY SYSTEM (Gate, Routing, Analysis)
         │
    NODE ONIONS (disposable, burn freely)
         │
    PROTECTED SITE
```

### 2.5.1 Node Onion Services
- [x] Each node gets its own .onion address
- [x] Separate Tor daemon for healthy node pool
- [x] Separate Tor daemon for threat node pool
- [x] Default: 10 healthy nodes, 3 threat nodes
- [x] Each node maintains its own circuit to protected backend

### 2.5.2 Node Lifecycle Management
- [x] Auto-spawn replacement nodes when one is burned
- [x] Load-balanced session assignment (round-robin to least-loaded node)
- [x] Maintain 70/30 healthy/threat ratio
- [x] Node health monitoring and auto-restart

### 2.5.3 Infection Detection & Burn Logic
- [x] Track session health per node
- [x] Auto-burn threshold: 60% bad sessions AND 10+ total sessions on node
- [x] Manual burn via admin panel
- [x] 24-hour grace period before node death (serve redirect page)

### 2.5.4 Grace Period Death Page
- [x] Static page when node is burned: "This route has been terminated"
- [x] Links to active public mirrors
- [x] Styled to match Fortify theme
- [x] Served for 24 hours, then node fully destroyed

### 2.5.5 Admin Panel Enhancements
- [x] Per-node session count and health percentage
- [x] Node onion addresses displayed
- [x] Manual burn button per node
- [x] Node pool status visualization

---

## Phase 3: Defensive Capabilities Enhancement (Partial ✅)

### 3.1 Adaptive Rate Limiting
**Current**: Fixed rate limits per tier
**Planned**: 
- [ ] Dynamic rate limits based on server load
- [ ] Per-path rate limiting
- [ ] Graduated slowdown before hard blocks
- [ ] Burst allowance for legitimate browsing

### 3.2 Circuit-Level Defense (Complete ✅)
- [x] **Vanguards Integration** - Layer 2/3 guard protection against guard discovery attacks
  - [x] VanguardsManager in Controller for lifecycle management
  - [x] Automatic start/stop with Controller
  - [x] Health monitoring with auto-restart on crash
  - [x] Attack alert parsing from vanguards logs
  - [x] Configurable via environment variables:
    - `VANGUARDS_ENABLED` - Enable/disable vanguards (default: true)
    - `VANGUARDS_LAYER2_GUARDS` - Number of layer2 guards (default: 4)
    - `VANGUARDS_LAYER3_GUARDS` - Number of layer3 guards (default: 8)
    - `VANGUARDS_CIRC_MAX_AGE_HOURS` - Circuit max age (default: 24)
    - `VANGUARDS_CIRC_MAX_MEGABYTES` - Circuit max data (default: 0/unlimited)
  - [x] Installation script: `install/vanguards_setup.sh`
  - [x] Configuration template: `install/templates/vanguards.conf.template`
- [ ] Tor circuit rotation detection
- [ ] Multi-circuit attack correlation
- [ ] Circuit-based trust scoring
- [ ] Guard node fingerprinting (privacy-preserving)

### 3.3 Tarpit/Delay Mechanisms
- [x] Progressive response delays for suspicious clients
- [x] Bandwidth throttling for threat tier
- [x] Resource exhaustion traps (honeypot endpoints)
- [ ] Fake content generation for scrapers

### 3.4 Proof-of-Work Challenges
**Goal**: Server-side verifiable challenges without JavaScript
- [x] CSS-based puzzle challenges (Direction arrows, Sequence patterns)
- [x] Form-based arithmetic challenges (Word unscramble)
- [x] Image selection challenges (Emoji, Silhouette identification)
- [x] Multiple captcha types with random cycling support
- [x] Separate threat captcha type for demoted users
- [x] Countdown timer with refresh option
- [ ] Time-locked challenges (must wait X seconds)

---

## Phase 4: Resilience & Recovery

### 4.1 Mirror Rotation
- [ ] Automatic mirror spawning on threat detection
- [ ] Graceful drain of burned mirrors
- [ ] DNS-like pointer system for mirror discovery
- [ ] Health scoring for active mirrors

### 4.2 Attack Logging & Forensics
- [ ] Structured attack logging (no PII)
- [ ] Attack pattern database
- [ ] Automated incident reports
- [ ] Historical trend analysis

### 4.3 Recovery Procedures
- [ ] Automatic service restart on crash
- [ ] State recovery after reboot
- [ ] Session persistence across restarts
- [ ] Graceful degradation modes

### 4.4 Session Continuity (NEW)
**Goal**: Allow users who pause their VM/browser to resume sessions without re-verification.

Users on Tor often pause VMs between browsing sessions. When they return hours later, their token is expired but they were a known-good session. Session Continuity maintains a lightweight history database to restore their previous trust status.

- [ ] Session history database (SQLite)
- [ ] History lookup on expired token
- [ ] Status transfer to new session ID
- [ ] 7-day maximum retention
- [ ] Killed/Burned sessions denied continuity
- [ ] Demotion count transfers, violation count resets
- [ ] Graceful degradation modes

---

## Phase 5: Fortify Cluster System

### 5.1 Multi-VPS Federation
**Goal**: Connect physically separated Fortify systems to protect the same service

- [ ] Secure inter-cluster communication via WireGuard tunnels
- [ ] Shared session state synchronization across cluster nodes
- [ ] Distributed threat intelligence sharing between instances
- [ ] Computational load sharing for PoW verification
- [ ] Mirror distribution across cluster (1 mirror per VPS minimum)
- [ ] Automatic failover when one cluster node goes down
- [ ] Cluster health monitoring and heartbeat system

### 5.2 Public Mirror Discovery Bar
- [ ] Slim header bar at top of Gate/intro pages
- [ ] Lists all public-facing mirrors across the cluster
- [ ] User can select alternative entry point
- [ ] Visual indicator of mirror health/load
- [ ] Styled to match site theme (or Fortify default)

### 5.3 Cluster Configuration
```toml
[cluster]
enabled = true
mode = "member"  # primary, member
cluster_name = "my-service-cluster"
wireguard_interface = "wg-fortify"
peer_addresses = [
  "10.0.0.1:51820",
  "10.0.0.2:51820"
]
shared_secret_path = "/etc/fortify/cluster.key"
sync_interval_ms = 1000
```

---

## Phase 6: Deployment Wizard

### 6.1 UI-Based Setup Experience
**Goal**: Replace manual configuration with guided deployment wizard

- [ ] Interactive terminal UI (TUI) or local web interface
- [ ] Step-by-step configuration flow
- [ ] Real-time validation of settings
- [ ] Preview of generated configuration

### 6.2 Deployment Modes
- [ ] **New Deployment**: Fresh install with full configuration
- [ ] **Wipe & Reinstall**: Clean slate while preserving identity keys
- [ ] **Join Cluster**: Connect to existing Fortify cluster
- [ ] **Upgrade**: Migrate from previous version

### 6.3 Branding & Customization
**Goal**: Allow operators to personalize their Fortify instance

- [ ] **Site Name**: Name of protected service (replaces "Fortify" branding)
- [ ] **Theme Selection**: Light theme / Dark theme
- [ ] **Custom Logo**: Upload site logo for Gate pages
- [ ] **Brand Colors**: Primary/accent color customization
- [ ] **Badge Shrinking**: Fortify branding minimized when site branding set

### 6.4 Vanity Address Generation
- [ ] Option to generate vanity .onion addresses for mirrors
- [ ] Character prefix specification (e.g., "mysite...")
- [ ] Background generation with progress indicator
- [ ] Multiple vanity addresses for different mirrors

### 6.5 Network Configuration
- [ ] Port selection for all components
- [ ] Bind address configuration
- [ ] Tor SOCKS/Control port settings
- [ ] WireGuard cluster network setup
- [ ] Firewall rule generation

### 6.6 Database & Backup Configuration
- [ ] Off-site backup destination (encrypted)
- [ ] Sync interval configuration
- [ ] State snapshot scheduling
- [ ] Recovery key generation

### 6.7 Security Configuration
- [ ] Rate limiting thresholds
- [ ] Captcha difficulty presets
- [ ] Trust tier timing
- [ ] Session token lifetime

### 6.8 Secrets Protection
**Critical**: Deployment must protect sensitive data

- [ ] **Zero Cleartext Secrets**: All secrets encrypted at rest
- [ ] **Memory Protection**: Secrets cleared from RAM when not in use
- [ ] **Physical Breach Resistance**: 
  - Encrypted secret storage with TPM/secure enclave if available
  - At minimum: protect real onion address of protected service
  - At minimum: protect IP addresses of cluster peers
- [ ] **Key Derivation**: Master password derives all encryption keys
- [ ] **Secure Wipe**: Panic button to zero all secrets

---

## Phase 7: Fast-Pass Identity System (Future - Low Priority)

**Goal**: Optional PGP-based persistent identity layer for returning users who wish to bypass CAPTCHA verification.

> **Note**: This system is designed to be dropped into the existing architecture with minimal changes. The core Fortify system should be fully functional before implementing Fast-Pass.

### 7.1 Core Concept

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         FAST-PASS FLOW                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   New User                              Returning Knight                    │
│      │                                        │                             │
│      ▼                                        ▼                             │
│   ┌──────────┐                         ┌──────────────┐                    │
│   │  Gate    │                         │  Gate        │                    │
│   │  CAPTCHA │                         │  PGP Sign    │                    │
│   └────┬─────┘                         └───────┬──────┘                    │
│        │                                       │                            │
│        ▼                                       ▼                            │
│   Start at UNKNOWN                      Start at TRUSTED                   │
│   (Tier 0)                              (Tier +4)                          │
│                                                                             │
│   Must earn trust                       Instant access                      │
│   through behavior                      (still demotable)                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 7.2 PGP Challenge System
- [ ] User registers PGP public key at Gate
- [ ] Gate stores key fingerprint (not full key) with profile
- [ ] On return: Gate presents random challenge string
- [ ] User signs challenge with private key
- [ ] Gate verifies signature against stored fingerprint
- [ ] Valid signature = instant session at Trusted tier

### 7.3 Freemium Tier Model

**Monetization via XMR (Monero) payments only - privacy-aligned**

| Feature | Squire (Free) | Knight (Paid) |
|---------|---------------|---------------|
| **Starting Tier** | Verified (+3) | Trusted (+4) |
| **CAPTCHA Bypass** | 1 easy captcha per session | Complete bypass |
| **Demotion Threshold** | Standard | Higher (more lenient) |
| **Priority Routing** | No | Yes (if QoS implemented) |
| **Key Rotation** | 1 per month | Unlimited |
| **Vouching Ability** | No | Yes |
| **Badge/Flair** | None | Knight badge |
| **Registration Limit** | Rate-limited (1 key/circuit/period) | Immediate |

### 7.4 Squire Tier (Free)
- [ ] PGP key registration allowed
- [ ] Start at Verified tier (+3) instead of Unknown
- [ ] Must complete 1 easy CAPTCHA per session (not threat captcha)
- [ ] Rate-limited registrations to prevent abuse
- [ ] Key rotation limited to 1 per month
- [ ] No vouching privileges
- [ ] Standard demotion thresholds apply

### 7.5 Knight Tier (Paid)
- [ ] XMR payment verification via payment proofs
- [ ] Start at Trusted tier (+4)
- [ ] Zero CAPTCHA - instant pass-through
- [ ] Higher demotion threshold (more forgiving)
- [ ] Unlimited key rotation (signed by old key)
- [ ] Vouching privileges (can sponsor new users)
- [ ] Priority routing when implemented
- [ ] Visual badge in any community features

### 7.6 Payment Integration
- [ ] XMR-only payment processing (privacy-aligned with Tor)
- [ ] Payment verification via view keys or payment proofs
- [ ] Payment ID linking to PGP key fingerprint
- [ ] Subscription tiers:
  - Monthly subscription
  - Yearly subscription (discount)
  - Lifetime access (one-time payment)
- [ ] Self-hosted payment processor integration
- [ ] Grace period for subscription renewals

### 7.7 Vouching System

**Purpose**: Knights can vouch for new users, sponsoring them to Squire status.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         VOUCHING FLOW                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   Knight (Paid)                                                            │
│       │                                                                     │
│       │ Generates voucher code                                             │
│       ▼                                                                     │
│   ┌─────────────┐                                                          │
│   │ Voucher     │  One-time use                                            │
│   │ Code: ABC12 │  Expires in 7 days                                       │
│   └──────┬──────┘                                                          │
│          │                                                                  │
│          │ Shared out-of-band                                              │
│          ▼                                                                  │
│   New User enters code at Gate                                             │
│          │                                                                  │
│          ▼                                                                  │
│   User prompted to register PGP key                                        │
│          │                                                                  │
│          ▼                                                                  │
│   User becomes Squire (free tier)                                          │
│   Knight's reputation linked to vouched user                               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Vouching Rules:**
- [ ] Knights can generate limited voucher codes (e.g., 3 per month)
- [ ] Voucher codes are one-time use, expire after 7 days
- [ ] Vouched users start as Squire (free tier)
- [ ] Knight's reputation is linked to vouched users
- [ ] If vouched user is demoted/killed, Knight loses vouching privileges temporarily
- [ ] Repeat abuse = permanent vouching ban for Knight
- [ ] Anti-abuse: Track behavioral fingerprints to prevent self-vouching

**Strategic Note**: Vouching is expected to be minimally used by legitimate users but may attract attackers who must pay for Knight status to abuse it - generating revenue while providing minimal attack surface.

### 7.8 Key Management
- [ ] Key rotation allowed (new key signed by old key)
- [ ] Revocation support (user can revoke own key)
- [ ] Expiration tracking (warn user before key expires)
- [ ] Multiple key support (different devices)
- [ ] Key fingerprint storage only (not full public key after initial verification)

### 7.9 Reputation & Demotion

**Fast-Pass users are NOT immune to behavioral analysis:**

- [ ] Bad behavior still triggers demotion
- [ ] Demoted Fast-Pass user must re-verify via PGP signature
- [ ] Repeated demotions accumulate on profile
- [ ] Threshold for "strike" system:
  - 3 demotions in 30 days = temporary suspension (7 days)
  - 5 demotions in 90 days = Knight → Squire downgrade
  - 10 demotions lifetime = permanent Fast-Pass revocation
- [ ] Admins can manually revoke Fast-Pass status

### 7.10 Reputation Decay (Optional)
- [ ] Knights who don't visit for X months may decay to Squire
- [ ] Squires who don't visit for X months may lose Fast-Pass entirely
- [ ] Configurable decay periods
- [ ] Re-activation via payment (Knight) or re-registration (Squire)

### 7.11 Anti-Abuse Measures
- [ ] Rate-limit new key registrations per Tor circuit
- [ ] Behavioral fingerprinting to detect multi-account abuse
- [ ] Voucher code honeypots to detect leaked/sold codes
- [ ] Payment pattern analysis (optional, privacy-conscious)
- [ ] Cross-reference demoted users' behavioral patterns with new registrations

### 7.12 Database Schema (Planned)

```rust
// Fast-Pass user profile
pub struct FastPassProfile {
    pub id: String,                          // UUID
    pub key_fingerprint: String,             // PGP key fingerprint
    pub tier: FastPassTier,                  // Squire or Knight
    pub created_at: SystemTime,
    pub last_seen: SystemTime,
    pub total_sessions: u64,
    pub demotion_count: u32,
    pub vouched_by: Option<String>,          // Profile ID of voucher
    pub vouched_users: Vec<String>,          // Profile IDs vouched for
    pub vouching_suspended_until: Option<SystemTime>,
    pub subscription_expires: Option<SystemTime>,  // None for Squire
    pub is_suspended: bool,
    pub is_revoked: bool,
}

pub enum FastPassTier {
    Squire,   // Free tier
    Knight,   // Paid tier
}

pub struct VoucherCode {
    pub code: String,
    pub issuer_profile_id: String,
    pub created_at: SystemTime,
    pub expires_at: SystemTime,
    pub used: bool,
    pub used_by: Option<String>,
}
```

### 7.13 Configuration (Planned)

```toml
[fast_pass]
enabled = false                              # Disabled by default
require_payment_for_knight = true            # Knight requires XMR payment

[fast_pass.squire]
starting_tier = "Verified"                   # +3
require_easy_captcha = true                  # 1 captcha per session
key_rotations_per_month = 1
registration_rate_limit_hours = 24           # 1 registration per 24h per circuit

[fast_pass.knight]
starting_tier = "Trusted"                    # +4
captcha_bypass = true
demotion_threshold_multiplier = 1.5          # 50% more lenient
vouchers_per_month = 3
voucher_expiry_days = 7

[fast_pass.xmr]
payment_address = "4..."                     # XMR address
view_key = "..."                             # For payment verification
monthly_price_xmr = "0.01"
yearly_price_xmr = "0.10"
lifetime_price_xmr = "0.50"

[fast_pass.reputation]
demotions_for_temp_suspension = 3
temp_suspension_days = 7
demotions_for_downgrade = 5
demotions_for_revocation = 10
decay_enabled = false
decay_period_days = 180
```

### 7.14 Gate UI Changes (Planned)
- [ ] "I have a Fast-Pass" button on Gate page
- [ ] PGP challenge input field
- [ ] "Register for Fast-Pass" link
- [ ] Registration form with PGP key paste
- [ ] Payment instructions (for Knight upgrade)
- [ ] Voucher code input field
- [ ] Status page showing profile info (demotions, subscription, etc.)

---

## Phase 8: Community Network

### 8.1 Federated Threat Intelligence
- [ ] Anonymous threat signature sharing
- [ ] Community blacklist federation
- [ ] Reputation exchange protocol
- [ ] Attack pattern propagation

### 8.2 Discovery Network
- [ ] Decentralized orchestrator discovery
- [ ] Mirror advertisement system
- [ ] Load sharing across community
- [ ] Trust-based peering

---

## Phase 9: Advanced Capabilities

### 9.1 Machine Learning Detection (Optional)
- [ ] Anomaly detection without data export
- [ ] Local model training
- [ ] Privacy-preserving pattern matching
- [ ] Adaptive threshold tuning

### 9.2 Integration Points
- [ ] Webhook alerts for attacks
- [ ] Prometheus/Grafana metrics export
- [ ] Syslog integration
- [ ] External blocklist import

### 9.3 Operational Tools
- [ ] CLI management interface
- [ ] Configuration hot-reload
- [ ] Rolling update support
- [ ] Backup/restore tooling

---

## Defensive Mechanism Ideas (Future Exploration)

### No-JS Challenge Concepts
1. **CSS Maze** - User must click through CSS-only navigation
2. **Form Timestamp** - Hidden form must mature before submission
3. **Cookie Dance** - Multi-step cookie exchange proving browser capability
4. **Header Challenge** - Server requests specific header patterns
5. **Referrer Chain** - Must visit pages in specific order

### Traffic Analysis Countermeasures
1. **Response Padding** - Normalize response sizes
2. **Timing Noise** - Add random delays to normalize timing
3. **Decoy Traffic** - Generate background noise
4. **Path Obfuscation** - Randomize internal routing

### Resource Protection
1. **Endpoint Hiding** - Sensitive paths behind additional gates
2. **Progressive Disclosure** - More content requires more trust
3. **Rate Budgets** - Per-session resource allocation
4. **Cost Attribution** - Track computational cost per client

---

## Non-Goals (Explicitly Out of Scope)

- ❌ Client-side JavaScript (ever)
- ❌ Offensive capabilities
- ❌ User tracking beyond sessions
- ❌ Data export to third parties
- ❌ Breaking Tor anonymity
- ❌ Storing PII

---

## Contributing

Contributions welcome for defensive mechanisms only. All code must:
- Build without warnings
- Include tests
- Follow existing patterns
- Document security implications
