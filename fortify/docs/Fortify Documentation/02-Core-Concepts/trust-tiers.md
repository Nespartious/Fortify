# 🔐 Trust Tier System

> **The Foundation of Fortify's Security Model**

---

## Overview

Fortify uses a **5-tier trust system** to classify and route user sessions. Each session starts at `Unknown` and can be promoted or demoted based on behavior.

---

## Trust Tiers

```
┌────────────────────────────────────────────────────────────────────────────────┐
│                           TRUST TIER HIERARCHY                                  │
├────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│    TIER          VALUE    ACCESS?    REQUIRES GATE?    ROUTING                 │
│    ──────────────────────────────────────────────────────────────────          │
│                                                                                 │
│    ┌─────────┐                                                                 │
│    │ TRUSTED │   +2      YES        NO                 Healthy Nodes           │
│    └────┬────┘                                         (Fast path)             │
│         │ promote()                                                            │
│    ┌────▼────┐                                                                 │
│    │VERIFIED │   +1      YES        NO                 Healthy Nodes           │
│    └────┬────┘                                         (Standard path)         │
│         │ demote()                                                             │
│    ┌────▼────┐                                                                 │
│    │ UNKNOWN │    0      NO         YES                Gate → Verify           │
│    └────┬────┘                                         (New users)             │
│         │ demote()                                                             │
│    ┌────▼─────┐                                                                │
│    │SUSPICIOUS│   -1      NO         YES                Gate → Re-verify       │
│    └────┬─────┘                                         (2 captchas)           │
│         │ demote()                                                             │
│    ┌────▼────┐                                                                 │
│    │ BURNED  │   -2      NO         N/A                Burned Page             │
│    └─────────┘                                         (Permanent)             │
│                                                                                 │
└────────────────────────────────────────────────────────────────────────────────┘
```

---

## Tier Definitions

### 🟢 TRUSTED (Value: +2)

**Description:** Sessions that have demonstrated consistent, legitimate behavior over time.

| Property | Value |
|----------|-------|
| `allows_access()` | `true` |
| `requires_gate()` | `false` |
| `can_promote()` | `false` (highest tier) |
| `can_demote()` | `true` |

**Routing:** Direct to Healthy Nodes with minimal inspection.

**Promotion Path:** Achieved through consistent good behavior (configurable threshold, default: 50 clean requests).

---

### 🔵 VERIFIED (Value: +1)

**Description:** Sessions that have completed captcha verification.

| Property | Value |
|----------|-------|
| `allows_access()` | `true` |
| `requires_gate()` | `false` |
| `can_promote()` | `true` → TRUSTED |
| `can_demote()` | `true` → SUSPICIOUS |

**Routing:** Standard path to Healthy Nodes.

**How to Achieve:**
1. Complete captcha challenge at Gate
2. Receive signed session token
3. Token is set in `fortify_session` cookie

---

### ⚪ UNKNOWN (Value: 0)

**Description:** New sessions with no verification history.

| Property | Value |
|----------|-------|
| `allows_access()` | `false` |
| `requires_gate()` | `true` |
| `can_promote()` | `true` → VERIFIED |
| `can_demote()` | `true` → SUSPICIOUS |

**Routing:** Redirected to Gate for verification.

**Typical Session:**
```
User connects → No token found → Unknown tier → Redirect to /Fortify
```

---

### 🟡 SUSPICIOUS (Value: -1)

**Description:** Sessions that have exhibited concerning behavior but haven't been fully burned.

| Property | Value |
|----------|-------|
| `allows_access()` | `false` |
| `requires_gate()` | `true` |
| `can_promote()` | `true` → VERIFIED (after re-verification) |
| `can_demote()` | `true` → BURNED |

**Routing:** Redirected to Gate with `is_threat=true`, requires **2 captchas**.

**Demotion Triggers:**
- 3+ violations at VERIFIED/TRUSTED tier
- 2+ violations at SUSPICIOUS tier
- Behavioral analysis threshold exceeded

---

### 🔴 BURNED (Value: -2)

**Description:** Permanently banned sessions.

| Property | Value |
|----------|-------|
| `allows_access()` | `false` |
| `requires_gate()` | N/A |
| `can_promote()` | `false` (permanent) |
| `can_demote()` | `false` (lowest tier) |

**Routing:** Served static "Burned" page, no further access.

**Burn Triggers:**
- 3 demotions (repeated pattern of being demoted and re-verified)
- Session marked as killed (persistent violator)
- Admin manual burn

---

## State Transitions

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           STATE TRANSITION DIAGRAM                               │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│                           ┌─────────────┐                                       │
│                           │   TRUSTED   │                                       │
│                           │    (+2)     │                                       │
│                           └──────┬──────┘                                       │
│                                  │                                               │
│                       3+ violations (Verified/Trusted)                          │
│                                  │                                               │
│      ┌───────────────────────────▼───────────────────────────┐                  │
│      │                                                        │                  │
│      │                    ┌─────────────┐                     │                  │
│      │   ┌────────────────│   VERIFIED  │────────────────┐    │                  │
│      │   │   50 clean     │    (+1)     │  3+ violations │    │                  │
│      │   │   requests     └──────┬──────┘                │    │                  │
│      │   │                       │                       │    │                  │
│      │   │                 Captcha                       │    │                  │
│      │   │                 Solved                        │    │                  │
│      │   │                       │                       │    │                  │
│      │   │                ┌──────▼──────┐                │    │                  │
│      │   └────────────────│   UNKNOWN   │                │    │                  │
│      │                    │    (0)      │                │    │                  │
│      │                    └──────┬──────┘                │    │                  │
│      │                           │                       │    │                  │
│      │                     Invalid                       │    │                  │
│      │                     behavior                      │    │                  │
│      │                           │                       │    │                  │
│      │   2 Captchas       ┌──────▼──────┐                │    │                  │
│      │   + Clean Slate────│  SUSPICIOUS │◄───────────────┘    │                  │
│      │                    │    (-1)     │                     │                  │
│      │                    └──────┬──────┘                     │                  │
│      │                           │                            │                  │
│      │                    3 demotions                        │                  │
│      │                    (persistent violator)              │                  │
│      │                           │                            │                  │
│      │                    ┌──────▼──────┐                     │                  │
│      │                    │   BURNED    │◄────────────────────┘                  │
│      │                    │    (-2)     │    3 demotions (kill threshold)       │
│      │                    └─────────────┘                                       │
│      │                                                                          │
│      └──────────────────────────────────────────────────────────────────────────┘
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Verification Requirements by Tier

| Starting Tier | Action Needed | Captchas Required | Result |
|---------------|---------------|-------------------|--------|
| Unknown | Pass Gate | 1 (normal) | Verified |
| Suspicious | Pass Gate | 2 (threat mode) | Verified |
| Verified (demoted) | Pass Gate | 2 (threat mode) | Verified (clean slate) |
| Burned | None available | N/A | Permanent block |

---

## Session Token Structure

```rust
pub struct SessionToken {
    pub session_id: String,      // UUID v4
    pub trust_tier: TrustTier,   // Current tier
    pub issued_at: u64,          // Unix timestamp
    pub expires_at: u64,         // Unix timestamp
    pub signature: Vec<u8>,      // HMAC-SHA256
}
```

### Token Lifecycle

1. **Creation:** Gate creates token after successful verification
2. **Signing:** HMAC-SHA256 with shared secret key
3. **Encoding:** Base64 encoded for cookie storage
4. **Validation:** HTTP Proxy validates on each request
5. **Expiration:** Default 1 hour (configurable)

### Token Cookie

```
Set-Cookie: fortify_session=<base64_token>; Path=/; HttpOnly; SameSite=Lax
```

---

## Administrative Overrides

Admins can manually set session tiers via the admin panel:

```rust
// Set tier override (persists until cleared)
admin_state.set_session_tier(session_id, "suspicious");

// Get override
let override_tier = admin_state.get_tier_override(session_id);

// Clear override (session returns to token tier)
admin_state.clear_tier_override(session_id);
```

**Important:** Fresh tokens (issued within 30 seconds) automatically clear tier overrides to allow demoted users to return to normal after re-verification.

---

## Demotion Count & Kill Threshold

Sessions track their demotion history:

```rust
pub struct SessionInfo {
    // ...
    pub demotion_count: u32,  // Times demoted and re-verified
    pub is_killed: bool,       // Permanently orphaned
}
```

| Demotion Count | Status | Description |
|----------------|--------|-------------|
| 0 | Normal | No demotions |
| 1-2 | Warning | Can still re-verify |
| 3+ | Killed | Repeat offender, permanently marked |

**Kill Threshold:** Configurable via `max_demotions_before_kill` (default: 3)

---

## Code Examples

### Checking Tier Properties

```rust
use fortify_core::TrustTier;

let tier = TrustTier::Verified;

// Check if allowed to access
if tier.allows_access() {
    // Route to backend
}

// Check if needs Gate
if tier.requires_gate() {
    // Redirect to Gate
}

// Check promotion eligibility
if tier.can_promote() {
    // Promote after good behavior
}
```

### Session State Transitions

```rust
let mut session = Session::new(token);

// Record violations
session.record_violation();
session.record_violation();
session.record_violation();

// Check if should demote
if session.should_demote() {
    session.demote()?;  // Verified → Suspicious
}

// Check if should burn
if session.should_burn() {
    session.burn();  // → Burned
}
```

---

## Trust Tier Comparison

```rust
// Tiers are comparable
assert!(TrustTier::Burned < TrustTier::Unknown);
assert!(TrustTier::Unknown < TrustTier::Verified);
assert!(TrustTier::Verified < TrustTier::Trusted);

// Sorting sessions by tier
sessions.sort_by_key(|s| s.token.trust_tier);
```

---

## Best Practices

1. **Don't skip tiers** - Demotion goes step by step (Trusted → Suspicious, not directly to Burned)

2. **Fresh tokens clear overrides** - Always check token age when promoting

3. **Monitor demotion counts** - High demotion counts indicate persistent attackers

4. **Use admin overrides sparingly** - Let the system handle most tier changes

5. **Configure thresholds appropriately** - Balance security vs user experience

---

## Session Continuity (Future Feature)

> **Status:** Planned - Medium Priority. Enables seamless session restoration for users who pause their VM/browser.

### The Problem

Many Tor users browse from virtual machines. They often:
1. Browse a site, earn Verified status
2. Pause the VM (not closing the browser)
3. Return hours or days later
4. Click a link on the still-open page
5. **Currently:** Forced back through CAPTCHA (token expired)
6. **With Continuity:** Seamlessly restored to previous status

### How It Works

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                      SESSION CONTINUITY FLOW                                     │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   BEFORE (Current Behavior)                                                     │
│   ─────────────────────────                                                      │
│                                                                                  │
│   User (Verified) → Pause VM → 12 hours → Resume → Token Expired → CAPTCHA!    │
│                                                                                  │
│   ══════════════════════════════════════════════════════════════════════════    │
│                                                                                  │
│   AFTER (With Session Continuity)                                               │
│   ───────────────────────────────                                                │
│                                                                                  │
│   User (Verified) → Pause VM → 12 hours → Resume → Token Expired                │
│                                                │                                 │
│                                                ▼                                 │
│                                     Session ID found in history                 │
│                                     Last status: Verified                       │
│                                     Age: 12 hours (< 7 days)                    │
│                                                │                                 │
│                                                ▼                                 │
│                                     New session issued                          │
│                                     Status: Verified (restored)                 │
│                                     NO CAPTCHA REQUIRED!                        │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Key Rules

| Rule | Description |
|------|-------------|
| **7-Day Maximum** | Session history expires after 7 days |
| **Status Transfer** | New session inherits last-known tier |
| **NOT Immunity** | Restored sessions still analyzed for threats |
| **Killed = Denied** | Killed sessions cannot use continuity |
| **Burned = Denied** | Burned sessions cannot use continuity |
| **New Session ID** | Always new ID, prevents replay attacks |
| **Demotion Count Transfers** | Bad history follows the user |
| **Violation Count Resets** | Fresh start on violations |

### What Gets Stored

```rust
pub struct SessionHistoryRecord {
    pub session_id: String,           // UUID
    pub last_trust_tier: TrustTier,   // Last known status
    pub demotion_count: u32,          // Carries over
    pub was_killed: bool,             // Permanent flag
    pub was_burned: bool,             // Permanent flag
    pub created_at: u64,              // Original creation
    pub last_seen_at: u64,            // Last activity
    pub expires_at: u64,              // 7 days from last_seen
    pub successor_id: Option<String>, // Link to new session
}
```

### What's NOT Stored (Privacy)

- ❌ IP addresses
- ❌ Request paths/URLs
- ❌ User agents
- ❌ Behavioral data
- ❌ Violation details

### Edge Cases

| Scenario | Result |
|----------|--------|
| Token expired, session in history, Verified | ✅ New Verified session |
| Token expired, session in history, Suspicious | ✅ New Suspicious session |
| Token expired, session was killed | ❌ Denied, treat as Unknown |
| Token expired, session not found | ❌ Normal Gate flow |
| Token expired, history > 7 days | ❌ Expired, Gate flow |
| Token valid (not expired) | ✅ Normal, no lookup needed |

### Planned Configuration

```toml
[session_continuity]
enabled = true
max_age_days = 7
storage_backend = "sqlite"
database_path = "/var/lib/fortify/sessions.db"

[session_continuity.transfer]
transfer_tier = true
transfer_demotion_count = true
reset_violation_count = true
deny_if_killed = true
deny_if_burned = true
```

---

## Fast-Pass Identity System (Future Feature)

> **Status:** Planned - Low Priority. The core Fortify system should be fully functional before implementing Fast-Pass.

### Overview

Fast-Pass is an optional **PGP-based persistent identity layer** that allows returning users to bypass CAPTCHA verification. Users who opt-in are called "Knights" (paid) or "Squires" (free).

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    STANDARD vs FAST-PASS AUTHENTICATION                          │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   STANDARD USER                          FAST-PASS USER                         │
│   ─────────────                          ──────────────                         │
│                                                                                  │
│   ┌─────────┐                            ┌─────────────┐                        │
│   │ Connect │                            │   Connect   │                        │
│   └────┬────┘                            └──────┬──────┘                        │
│        │                                        │                               │
│        ▼                                        ▼                               │
│   ┌─────────┐                            ┌─────────────┐                        │
│   │ CAPTCHA │                            │  PGP Sign   │                        │
│   │ Verify  │                            │  Challenge  │                        │
│   └────┬────┘                            └──────┬──────┘                        │
│        │                                        │                               │
│        ▼                                        ▼                               │
│   Start at                               Squire: Start at                       │
│   UNKNOWN (0)                            VERIFIED (+1)                          │
│                                          + 1 easy captcha                       │
│                                                                                  │
│                                          Knight: Start at                       │
│                                          TRUSTED (+2)                           │
│                                          + NO captcha                           │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Fast-Pass Tiers

| Tier | Cost | Starting Trust | Captcha | Demotion Threshold | Vouching |
|------|------|----------------|---------|-------------------|----------|
| **Squire** | Free | Verified (+1) | 1 easy per session | Standard | No |
| **Knight** | XMR | Trusted (+2) | None | 1.5x (more lenient) | Yes |

### PGP Challenge Flow

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                         PGP AUTHENTICATION FLOW                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   1. User clicks "I have a Fast-Pass" at Gate                                   │
│                      │                                                          │
│                      ▼                                                          │
│   2. Gate presents random challenge string                                      │
│      Challenge: "fortify-challenge-a7b3c9d2e1f0..."                            │
│                      │                                                          │
│                      ▼                                                          │
│   3. User signs challenge with PGP private key                                  │
│      (using GPG, OpenPGP.js, or compatible tool)                               │
│                      │                                                          │
│                      ▼                                                          │
│   4. User submits signature to Gate                                             │
│                      │                                                          │
│                      ▼                                                          │
│   5. Gate verifies signature against stored fingerprint                         │
│                      │                                                          │
│            ┌─────────┴─────────┐                                               │
│            │                   │                                                │
│         VALID              INVALID                                             │
│            │                   │                                                │
│            ▼                   ▼                                                │
│      Session created     Fall back to                                          │
│      at tier based on    standard CAPTCHA                                      │
│      Fast-Pass level                                                           │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Registration Process

**For Squire (Free):**
1. User visits Gate registration page
2. Pastes PGP public key
3. Fortify stores key fingerprint (not full key)
4. Rate-limited: 1 registration per Tor circuit per 24 hours
5. Key rotation: 1 per month

**For Knight (Paid):**
1. Complete Squire registration
2. Generate XMR payment address
3. User sends XMR payment
4. Payment verified via view key/payment proof
5. Account upgraded to Knight
6. Subscription tracked (monthly/yearly/lifetime)

### Fast-Pass Users Are Still Demotable

**Critical:** Fast-Pass does NOT grant immunity from behavioral analysis.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    FAST-PASS DEMOTION CONSEQUENCES                               │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   DEMOTION EVENT                         CONSEQUENCE                            │
│   ─────────────────────────────────────────────────────────────────────         │
│                                                                                  │
│   1st demotion                           Must re-verify via PGP signature       │
│                                          (no permanent penalty)                 │
│                                                                                  │
│   3 demotions in 30 days                 Temporary suspension (7 days)          │
│                                          Cannot use Fast-Pass                   │
│                                                                                  │
│   5 demotions in 90 days                 Knight → Squire downgrade              │
│                                          Lose paid benefits                     │
│                                                                                  │
│   10 demotions lifetime                  Permanent Fast-Pass revocation         │
│                                          Must use standard CAPTCHA              │
│                                                                                  │
│   Admin manual revoke                    Immediate Fast-Pass revocation         │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Vouching System (Knight Only)

Knights can vouch for new users, sponsoring them directly to Squire status:

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           VOUCHING MECHANISM                                     │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   Knight generates voucher code                                                 │
│         │                                                                        │
│         │  Code: "KNIGHT-A7B3-C9D2"                                             │
│         │  Expires: 7 days                                                       │
│         │  One-time use                                                         │
│         │                                                                        │
│         ▼                                                                        │
│   Knight shares code out-of-band                                                │
│   (encrypted chat, etc.)                                                        │
│         │                                                                        │
│         ▼                                                                        │
│   New user enters code at Gate                                                  │
│         │                                                                        │
│         ▼                                                                        │
│   New user registers PGP key                                                    │
│         │                                                                        │
│         ▼                                                                        │
│   New user becomes Squire                                                       │
│   Knight's reputation linked                                                    │
│                                                                                  │
│   ═══════════════════════════════════════════════════════════════════════       │
│   ABUSE CONSEQUENCES                                                            │
│   ═══════════════════════════════════════════════════════════════════════       │
│                                                                                  │
│   Vouched user demoted    →    Knight warned                                    │
│   Vouched user killed     →    Knight loses vouching (temporary)                │
│   Multiple abuses         →    Knight loses vouching (permanent)                │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Vouching Limits:**
- 3 voucher codes per month (Knight tier)
- Codes expire after 7 days
- One-time use only
- Knight reputation is linked to vouched users

**Strategic Note:** The vouching system is expected to see minimal legitimate use but may attract attackers who must first pay for Knight status to abuse it - generating revenue while providing minimal attack surface.

### Monetization Model (XMR Only)

**Payment Options:**

| Plan | Price (Example) | Duration |
|------|-----------------|----------|
| Monthly | 0.01 XMR | 30 days |
| Yearly | 0.10 XMR | 365 days |
| Lifetime | 0.50 XMR | Forever |

**Why XMR (Monero)?**
- Privacy-aligned with Tor service users
- Untraceable transactions
- No KYC requirements
- Self-hostable verification via view keys

### Key Management

| Feature | Squire | Knight |
|---------|--------|--------|
| Key Rotation | 1 per month | Unlimited |
| Multiple Keys | No | Yes (different devices) |
| Key Revocation | Self-revoke only | Self-revoke only |
| Fingerprint Storage | Yes | Yes |

**Key Rotation:** New key must be signed by old key to prove ownership.

### Reputation Decay (Optional)

If enabled, inactive Fast-Pass users may decay:

| Inactivity Period | Consequence |
|-------------------|-------------|
| 90 days (Knight) | Warning notification |
| 180 days (Knight) | Decay to Squire |
| 180 days (Squire) | Fast-Pass revoked |

**Re-activation:** Payment (Knight) or re-registration (Squire)

### Anti-Abuse Measures

1. **Rate Limiting:** 1 key registration per Tor circuit per 24 hours
2. **Behavioral Fingerprinting:** Detect multi-account abuse patterns
3. **Voucher Honeypots:** Detect leaked/sold voucher codes
4. **Cross-Reference:** Compare demoted user patterns with new registrations
5. **Demotion Tracking:** Persistent record across PGP-linked sessions

### Planned Data Structures

```rust
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
    pub subscription_expires: Option<SystemTime>,  // None = lifetime or Squire
    pub is_suspended: bool,
    pub is_revoked: bool,
}

pub enum FastPassTier {
    Squire,   // Free tier - Verified (+1) + 1 easy captcha
    Knight,   // Paid tier - Trusted (+2) + no captcha
}
```

### Planned Configuration

```toml
[fast_pass]
enabled = false                              # Disabled by default

[fast_pass.squire]
starting_tier = "Verified"                   # +1
require_easy_captcha = true
key_rotations_per_month = 1
registration_rate_limit_hours = 24

[fast_pass.knight]
starting_tier = "Trusted"                    # +2
captcha_bypass = true
demotion_threshold_multiplier = 1.5          # 50% more lenient
vouchers_per_month = 3
voucher_expiry_days = 7

[fast_pass.xmr]
payment_address = "4..."
view_key = "..."
monthly_price_xmr = "0.01"
yearly_price_xmr = "0.10"
lifetime_price_xmr = "0.50"

[fast_pass.reputation]
demotions_for_temp_suspension = 3
demotions_for_downgrade = 5
demotions_for_revocation = 10
decay_enabled = false
decay_period_days = 180
```

---

*See [behavioral-analysis.md](behavioral-analysis.md) for violation details*
