# XMR Transaction-Based Gate Entry Verification

**Status**: 📋 PLANNING  
**Priority**: MEDIUM  
**Complexity**: HIGH  
**Created**: January 23, 2026

---

## Executive Summary

An optional system that allows users to bypass CAPTCHA verification during high-traffic/DDoS scenarios by paying a small XMR fee. Verified transactions grant extended access (168 hours) with direct proxy access to the protected service.

---

## Problem Statement

During large-scale DDoS attacks or traffic spikes:
- Legitimate users face difficulty completing CAPTCHA verification
- Rate limiting affects both attackers and real users
- CAPTCHA pool may be exhausted
- User frustration leads to abandonment

### Current User Experience During Attack

```
User → Gate → CAPTCHA → (rate limited) → retry → (timeout) → frustration → leave
```

### Proposed Alternative Path

```
User → Gate → XMR Payment Option → Pay Small Fee → Instant Verification → Direct Access (168h)
```

---

## Proposed Solution

### High-Level Flow

1. **User hits Gate/CAPTCHA page** during congestion
2. **Option displayed**: "Skip verification with XMR payment (0.0001 XMR)"
3. **User scans QR code** with XMR wallet
4. **Backend monitors for transaction** (subaddress per session)
5. **On confirmation**: Session elevated to `Trusted` tier, 168h validity
6. **Session token persisted**: User can reconnect with same token

### Key Features

| Feature | Description |
|---------|-------------|
| **Optional** | Never mandatory, always an alternative path |
| **Tiny Fee** | ~$0.01-0.05 USD equivalent (symbolic, not profit) |
| **168h Validity** | Verified status persists for 7 days |
| **Unique Subaddress** | Each session gets unique XMR subaddress |
| **Automatic Verification** | No manual intervention required |
| **Reconnection Token** | User can re-enter with stored token |

---

## Technical Architecture

### Components Required

```
┌─────────────────────────────────────────────────────────────────┐
│                     XMR GATE ENTRY SYSTEM                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐    ┌──────────────────┐    ┌──────────────┐  │
│  │   Gate/UI    │    │  XMR Verifier    │    │  Monero RPC  │  │
│  │              │───▶│   Service        │───▶│  (Local/     │  │
│  │ Payment Page │    │                  │    │   Remote)    │  │
│  └──────────────┘    └──────────────────┘    └──────────────┘  │
│         │                     │                                  │
│         ▼                     ▼                                  │
│  ┌──────────────┐    ┌──────────────────┐                       │
│  │ Session Mgr  │◀───│  Payment DB      │                       │
│  │ (Trust Tier) │    │  (Pending/Done)  │                       │
│  └──────────────┘    └──────────────────┘                       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### New Components

1. **XMR Verifier Service** (Rust crate: `fortify-xmr`)
   - Connects to Monero wallet RPC
   - Generates unique subaddresses per session
   - Monitors for incoming transactions
   - Confirms payment and notifies Gate

2. **Payment Database** (SQLite/in-memory)
   - Maps session_id → subaddress
   - Tracks payment status (pending/confirmed/expired)
   - Stores reconnection tokens

3. **Gate UI Extension**
   - Payment option button
   - QR code display
   - Real-time confirmation status
   - Reconnection token display/input

---

## User Experience Flow

### Initial Payment Flow

```
1. User arrives at Gate page
2. Sees: "Having trouble? Skip with XMR payment"
3. Clicks button → Modal shows:
   - QR code with subaddress
   - Amount: 0.0001 XMR
   - Expiration: 10 minutes
   - "Waiting for payment..."
4. User sends payment from wallet
5. After 1-2 confirmations:
   - Modal updates: "Payment confirmed!"
   - Shows reconnection token
   - Auto-redirects to backend
6. User accesses service for 168 hours
```

### Reconnection Flow

```
1. User returns to Gate after initial session expired
2. Instead of CAPTCHA, enters reconnection token
3. Token validated → Immediate access
4. Works for 168h from original payment
```

---

## Discussion Points

### 1. Is This Worth Implementing?

**Arguments FOR:**
- Provides legitimate users a guaranteed entry path during attacks
- Tiny fee creates economic barrier for attackers
- Monero ensures payment privacy (no deanonymization)
- Revenue can offset infrastructure costs
- Differentiates Fortify from competitors

**Arguments AGAINST:**
- Adds significant complexity
- Requires Monero node/wallet infrastructure
- Creates "pay-to-access" perception concerns
- Additional attack surface (XMR service)
- May not align with all use cases

### 2. Fee Structure

| Option | Amount | Pros | Cons |
|--------|--------|------|------|
| **Symbolic** | 0.0001 XMR (~$0.01) | Low barrier, anti-spam | Minimal revenue |
| **Low** | 0.001 XMR (~$0.10) | Better deterrence | May exclude some users |
| **Configurable** | Operator sets | Flexible | More complexity |

**Recommendation**: Start symbolic, make configurable.

### 3. Duration of Verification

| Option | Duration | Pros | Cons |
|--------|----------|------|------|
| **24 hours** | 1 day | Fresh verification | Frequent payments |
| **168 hours** | 7 days | Convenient | Longer attack window |
| **Configurable** | Operator sets | Flexible | More settings |

**Recommendation**: 168 hours default, configurable 24h-720h.

### 4. Transaction Confirmation Requirements

| Option | Confirmations | Wait Time | Security |
|--------|---------------|-----------|----------|
| **0-conf** | 0 | Instant | Low (double-spend risk) |
| **1-conf** | 1 | ~2 min | Medium |
| **2-conf** | 2 | ~4 min | High |
| **Configurable** | Operator | Variable | Flexible |

**Recommendation**: 1 confirmation default (balance speed/security).

### 5. Reconnection Token Security

- Token should be cryptographically secure (256-bit random)
- Token tied to session fingerprint (prevent sharing)
- Token expires with verification period
- Rate limit token attempts (prevent brute force)

### 6. Infrastructure Requirements

**Option A: Local Monero Wallet**
- Full control
- No third-party dependency
- Higher resource usage (~4GB RAM)
- Requires blockchain sync

**Option B: Remote Monero Node**
- Lower resource usage
- Faster setup
- Trust third-party node
- May affect privacy

**Recommendation**: Support both, default to remote with option for local.

---

## Security Considerations

### Potential Attacks

| Attack | Mitigation |
|--------|------------|
| **Double-spend** | Require confirmations |
| **Token stealing** | Fingerprint binding |
| **Token brute force** | Rate limiting |
| **Payment spoofing** | Verify on-chain |
| **Subaddress collision** | Cryptographic generation |
| **Service DoS** | Separate XMR service |

### Privacy Considerations

- XMR subaddresses are unlinkable
- No user identity stored
- Payment history not tied to sessions
- Token does not reveal payment info

---

## Implementation Phases

### Phase 1: Proof of Concept
- Basic Monero RPC integration
- Static subaddress generation
- Manual verification testing
- ~20 hours development

### Phase 2: Core Integration
- Session-linked subaddresses
- Auto-verification
- Gate UI integration
- ~40 hours development

### Phase 3: Production Ready
- Reconnection tokens
- Configurable settings
- Monitoring/alerting
- Documentation
- ~30 hours development

**Total Estimated Effort**: 90-120 hours

---

## Decision Required

### Go/No-Go Criteria

Before implementing, answer:

1. **Does your use case benefit from paid bypass?**
   - High-traffic services under frequent attack: YES
   - Small personal projects: PROBABLY NOT

2. **Can you maintain Monero infrastructure?**
   - Need wallet + optional node
   - Ongoing maintenance required

3. **Are your users comfortable with crypto payments?**
   - Dark web services: LIKELY YES
   - Mainstream services: MAYBE NOT

4. **Is the complexity worth it?**
   - Adds significant codebase surface
   - Another component to monitor

---

## Alternatives Considered

### 1. Proof-of-Work CAPTCHA
- User's browser does computational work
- No payment required
- Less reliable than payment

### 2. Email Verification
- Requires email collection (privacy concern)
- Slower than XMR payment
- Not suitable for anonymous services

### 3. Invite Codes
- Existing users invite new users
- No payment required
- Requires user base first

### 4. Time-Based Rate Limiting
- First N users per minute get through
- No payment required
- Unfair (fastest wins)

---

## Next Steps

1. [ ] Team discussion on go/no-go
2. [ ] If go: Create technical RFC
3. [ ] If go: Prototype XMR RPC integration
4. [ ] If go: UI/UX mockups
5. [ ] If go: Security review of design

---

## References

- [Monero Wallet RPC Documentation](https://www.getmonero.org/resources/developer-guides/wallet-rpc.html)
- [Monero Subaddresses](https://www.getmonero.org/resources/moneropedia/subaddress.html)
- [Payment Verification Best Practices](https://www.getmonero.org/resources/merchant-guide/)

---

## Appendix: Example Config

```toml
[xmr_gate]
enabled = false
wallet_rpc_address = "http://127.0.0.1:18083"
payment_amount = 0.0001  # XMR
verification_duration_hours = 168
confirmations_required = 1
payment_timeout_minutes = 10
reconnection_tokens_enabled = true
```

