# Trust Levels

## Overview

Fortify implements a progressive trust system where sessions earn higher trust through verified behavior. Trust affects routing, rate limits, and verification requirements.

## Trust Tiers

### Tier 0: Unknown
**Initial state for all new connections**

- **Characteristics**:
  - No prior interaction
  - Zero trust
  - Maximum scrutiny

- **Requirements**:
  - Must complete Gate challenges
  - Server-side captcha
  - Proof-of-work computation
  - Rate limited aggressively

- **Access**:
  - Gate only
  - No service access until verified

- **Duration**: Until verification complete

### Tier 1: Verified
**Successfully completed Gate challenges**

- **Characteristics**:
  - Human verification passed
  - Computational investment made
  - Token issued

- **Requirements**:
  - Valid signed token
  - Token not expired
  - No suspicious behavior

- **Access**:
  - Fast-path HTTP proxy
  - Routed to Healthy nodes
  - Standard rate limits

- **Duration**: Token lifetime (configurable, typically 1-24 hours)

### Tier 2: Trusted
**Sustained good behavior over time**

- **Characteristics**:
  - Multiple successful sessions
  - No violations
  - Consistent patterns

- **Requirements**:
  - Token refresh without re-verification
  - Extended token lifetime
  - Lower inspection overhead

- **Access**:
  - Priority routing
  - Higher rate limits
  - Reduced captcha frequency

- **Duration**: Extended lifetime (24-72 hours)

### Tier -1: Suspicious
**Triggered behavioral heuristics**

- **Characteristics**:
  - Anomalous request patterns
  - Unusual timing
  - Header inconsistencies
  - Rate limit violations

- **Requirements**:
  - Silent demotion
  - Routed to Threat nodes
  - Increased inspection

- **Access**:
  - Service access maintained
  - Additional verification on sensitive actions
  - Reduced rate limits

- **Duration**: Until behavior normalizes or expires

### Tier -2: Burned
**Definitive hostile behavior**

- **Characteristics**:
  - Attack patterns detected
  - Token forgery attempt
  - Repeated violations
  - Known bad actor

- **Requirements**:
  - Hard rejection
  - Token revoked
  - IP/fingerprint banned

- **Access**:
  - None
  - Must re-verify from new origin

- **Duration**: Configurable ban period (1-72 hours, or permanent)

## Trust Transitions

### Promotion Paths

```
Unknown → Verified:  Complete Gate challenges
Verified → Trusted:  Sustained good behavior
Suspicious → Verified: Behavior normalizes
```

### Demotion Paths

```
Verified → Suspicious:   Heuristic triggers
Trusted → Suspicious:    Anomaly detected
Verified → Burned:       Attack detected
Suspicious → Burned:     Repeated violations
```

## Session Tokens

### Token Structure
- **Issuer**: Gate component
- **Signature**: HMAC or Ed25519
- **Claims**:
  - Session ID
  - Trust tier
  - Issue timestamp
  - Expiration timestamp
  - Fingerprint hash (optional)

### Token Validation
1. Signature verification
2. Expiration check
3. Revocation list check (if maintained)
4. Trust tier extraction

### Token Refresh
- **Verified → Verified**: Minimal re-verification (PoW only)
- **Trusted → Trusted**: Automatic refresh if behavior good
- **Suspicious**: No refresh until promotion

## Behavioral Heuristics

### Trigger Suspicious
- Request rate spikes
- Unusual HTTP methods
- Malformed headers
- Path traversal attempts
- SQL injection patterns
- Known scanner signatures

### Trigger Burned
- Token forgery attempt
- Repeated automated behavior
- Replay attacks
- Multiple violations from Suspicious state

## Privacy Considerations

- **No persistent identifiers**: Tokens are session-specific
- **No logging of content**: Only metadata for decisions
- **No cross-session correlation**: Unless fingerprint match
- **Silent demotion**: Users not informed of Suspicious state

## Implementation Notes

### Trust State Storage
- In-memory SessionManager with HashMap
- Thread-safe with Arc<Mutex<>>
- Sessions expire automatically
- LRU cleanup for idle sessions

### Token Signing
- HMAC-SHA256 for token signatures
- Base64-encoded JSON payloads
- Configurable secret key per deployment
- Constant-time signature verification

### Session Lifecycle
```rust
// Creation
let token = SessionToken::new(session_id, TrustTier::Unknown, 3600);
token.sign(secret_key)?;

// Validation
token.verify(secret_key)?;
if token.is_expired() { /* reject */ }

// Promotion
session.promote()?; // Unknown -> Verified

// Demotion on violations
session.record_violation();
if session.should_demote() {
    session.demote()?; // Verified -> Suspicious
}

// Burn on repeated violations
if session.should_burn() {
    session.burn(); // Any -> Burned
}
```

### Concurrency
- Thread-safe token validation
- Lock-free tier checks where possible
- Per-session state isolation
- SessionManager handles concurrent access

### Violation Thresholds
- Verified/Trusted: 3 violations → demote
- Suspicious: 2 violations → demote
- Any tier: 10 violations → burn
- Suspicious: 5 violations → burn

### Failure Modes
- **Unknown state**: Default to Unknown tier
- **Validation failure**: Demote to Unknown
- **Service unavailable**: Reject at Gate

## Phase 2 Implementation Status

### Completed Features
- ✓ HMAC-SHA256 token signing and verification
- ✓ Trust tier state machine with transitions
- ✓ Session lifecycle management
- ✓ Promotion/demotion logic with violation tracking
- ✓ Token expiration and validation
- ✓ Base64 token encoding/decoding
- ✓ SessionManager for in-memory storage
- ✓ Thread-safe session operations
- ✓ Comprehensive unit tests (15+ tests)
- ✓ Violation thresholds and burn logic
- ✓ Idle session detection

### Token Security
- Secret key must be at least 32 bytes
- Tokens include timestamp to prevent replay
- Signature covers all token fields
- No persistent token storage (memory only)
- Automatic cleanup of expired tokens
