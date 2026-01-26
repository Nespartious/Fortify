# 🛡️ Threat Model

> **Understanding the Attacks Fortify Defends Against**

---

## Overview

Fortify is designed to protect Tor hidden services from a wide range of attacks while maintaining user privacy and service availability. This document describes the threat model, attack scenarios, and defensive strategies.

---

## Threat Actors

### Script Kiddies (Low Skill)
**Capabilities:**
- Pre-built DDoS tools
- Basic web scrapers
- Automated vulnerability scanners

**Attack Vectors:**
- HTTP flood (many simple requests)
- Directory scanning (`/admin`, `/wp-admin`, etc.)
- Path traversal attempts (`../../../etc/passwd`)

**Fortify Defense:**
- CAPTCHA verification blocks automated tools
- Behavioral analysis detects scanning patterns
- Rate limiting prevents floods

---

### Motivated Attackers (Medium Skill)
**Capabilities:**
- Custom attack scripts
- Distributed botnets
- Protocol-level attacks
- Session manipulation

**Attack Vectors:**
- Distributed HTTP floods
- CAPTCHA solving services
- Cookie/session token manipulation
- Tor circuit abuse

**Fortify Defense:**
- Circuit-based rate limiting (each circuit gets quota)
- Multi-CAPTCHA for suspicious sessions
- HMAC-signed tokens (cannot be forged)
- Demotion system (repeat offenders get burned)

---

### Advanced Persistent Threats (High Skill)
**Capabilities:**
- Zero-day exploits
- Guard discovery attacks
- Timing attacks
- Resource exhaustion

**Attack Vectors:**
- Guard node enumeration
- Circuit fingerprinting
- Memory exhaustion
- Slow-loris attacks

**Fortify Defense:**
- Vanguards (Layer 2/3 guard protection)
- Timeout protection on all network operations
- Safe lock helpers (prevent cascading failures)
- Mirror burn capability (can replace compromised entry points if needed)

---

## Attack Scenarios

### Scenario 1: HTTP Flood DDoS

**Attack:**
```
Attacker spins up 1,000 Tor circuits
Each circuit makes 100 requests/second
Total: 100,000 req/sec overwhelming backend
```

**Defense:**
1. **Initial Block:** Unknown sessions redirected to Gate (CAPTCHA)
2. **Circuit Quotas:** Each circuit limited to 10 req/10sec
3. **Result:** Attack traffic capped at 1,000 req/10sec = 100 req/sec
4. **Real Users:** Independent quotas, unaffected

**Outcome:** ✅ Attack mitigated, service remains available

---

### Scenario 2: CAPTCHA Solving Service

**Attack:**
```
Attacker uses paid CAPTCHA solving service
Solves CAPTCHAs to get VERIFIED tokens
Uses tokens for targeted attacks
```

**Defense:**
1. **Behavioral Analysis:** Detects bot-like patterns (even with valid token)
2. **Demotion:** Session demoted to SUSPICIOUS after 3 violations
3. **Multi-CAPTCHA:** Now requires 2 CAPTCHAs (harder difficulty)
4. **Kill Threshold:** After 3 demotions → permanently burned

**Outcome:** ✅ Economically unfeasible (each session costs $ and gets burned quickly)

---

### Scenario 3: Guard Discovery Attack

**Attack:**
```
Attacker creates many circuits to your service
Analyzes circuit timing to discover guard nodes
Uses guard node info for targeted attacks
```

**Defense:**
1. **Vanguards:** Layer 2/3 guard pinning prevents guard discovery
2. **Circuit Rotation:** Circuits rotate automatically
3. **Mirror Rotation:** Public entry points rotate, not just circuits
4. **Attack Detection:** Vanguards logs anomalies

**Outcome:** ✅ Guard nodes remain hidden, attack surfaces minimized

---

### Scenario 4: Resource Exhaustion

**Attack:**
```
Attacker opens many connections
Never completes requests (slow-loris)
Exhausts server connection pool
```

**Defense:**
1. **TCP Handshake Timeout:** 10 seconds
2. **Header Read Timeout:** 30 seconds  
3. **Request Timeout:** 60 seconds
4. **Connection Limits:** Max concurrent connections enforced

**Outcome:** ✅ Incomplete connections closed automatically

---

### Scenario 5: Session Token Forgery

**Attack:**
```
Attacker captures a VERIFIED token
Tries to modify trust tier or expiration
Attempts to create fake tokens
```

**Defense:**
1. **HMAC-SHA256 Signing:** Token includes signature
2. **Signature Verification:** Any modification invalidates token
3. **Server-Side State:** Session state stored server-side
4. **Token Binding:** Token tied to specific session ID

**Outcome:** ✅ Forged tokens rejected, attacker must re-verify

---

### Scenario 6: Mirror Compromise

**Attack:**
```
Attacker discovers a mirror .onion address
Attempts to correlate traffic patterns
Tries to identify backend service
```

**Defense:**
1. **Long-lived Mirrors:** Mirrors stable over months
2. **Burn Capability:** Compromised mirrors CAN be destroyed if needed
3. **Multiple Active Mirrors:** No single point of failure
4. **Traffic Isolation:** Backend never exposed

**Outcome:** ✅ Compromised mirror can be burned if necessary, service continues

---

## Attack Surface Analysis

```
┌────────────────────────────────────────────────────────────────┐
│                      ATTACK SURFACE                             │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│  COMPONENT          EXPOSURE    HARDENING                      │
│  ──────────────────────────────────────────────────────        │
│                                                                 │
│  Public Mirrors     HIGH        • Rotate regularly             │
│                                 • Burn on compromise           │
│                                 • Multiple active              │
│                                                                 │
│  HTTP Proxy         HIGH        • Token validation             │
│                                 • Behavioral analysis          │
│                                 • Rate limiting                │
│                                 • Timeout protection           │
│                                                                 │
│  Gate               HIGH        • CAPTCHA verification         │
│                                 • Cookie compliance            │
│                                 • Rate limiting                │
│                                                                 │
│  Nodes              MEDIUM      • Isolated circuits            │
│                                 • Deep inspection              │
│                                 • Auto-burn on infection       │
│                                                                 │
│  Controller         LOW         • Internal only                │
│                                 • No external exposure         │
│                                                                 │
│  Backend            NONE        • Never exposed                │
│                                 • Isolated .onion              │
│                                 • Accessed only via nodes      │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

---

## Security Assumptions

### What Fortify Assumes

| Assumption | Rationale |
|-----------|-----------|
| Tor network is trustworthy | Using official Tor Project software |
| Server OS is secure | Hardening scripts provided |
| Admin credentials protected | Authentication system in place |
| Tor Browser users legitimate | Safest mode users are likely real humans |

### What Fortify Does NOT Assume

| Non-Assumption | Mitigation |
|----------------|------------|
| All users are friendly | Default deny, explicit verification required |
| Guard nodes are safe | Vanguards protection against guard discovery |
| Mirrors won't be compromised | Burn & replace strategy |
| Network is reliable | Timeout protection, graceful degradation |
| Locks never poison | Safe lock helpers |

---

## Residual Risks

### Known Limitations

1. **Zero-Day Exploits**
   - **Risk:** Unknown vulnerabilities in Rust, Tor, or dependencies
   - **Mitigation:** Regular updates, fuzzing infrastructure (planned)

2. **Social Engineering**
   - **Risk:** Admin credentials compromised
   - **Mitigation:** Strong passwords, session management, audit logging

3. **Nation-State Attacks**
   - **Risk:** Traffic correlation, timing attacks, compromised Tor nodes
   - **Mitigation:** Vanguards, circuit isolation, best practices documentation

4. **Economic Attacks**
   - **Risk:** Well-funded attacker with unlimited CAPTCHA solving budget
   - **Mitigation:** Demotion system makes sustained attacks expensive

---

## Security Principles

### Defense in Depth

Fortify implements multiple layers of protection:
1. **Entry Layer:** CAPTCHA verification
2. **Routing Layer:** Trust-based traffic segregation
3. **Detection Layer:** Behavioral analysis
4. **Response Layer:** Demotion and burning
5. **Network Layer:** Vanguards and circuit isolation
6. **Infrastructure Layer:** Multiple stable mirrors

### Fail Closed

When in doubt, Fortify denies access:
- Unknown sessions → Gate (not backend)
- Invalid tokens → Re-verification required
- Errors → Service degradation, not compromise

### Privacy First

Security measures respect Tor's privacy model:
- No cross-session correlation
- No IP-based tracking
- No browser fingerprinting
- JavaScript-free (compatible with Safest mode)

---

*See [attack-mitigations.md](attack-mitigations.md) for detailed mitigation strategies*
