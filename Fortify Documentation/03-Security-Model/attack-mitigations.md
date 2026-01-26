# 🎯 Attack Mitigations

> **Comprehensive Defense Strategies for Each Attack Vector**

---

## DDoS Attacks

### HTTP Flood

**Attack Description:**
Overwhelming the service with massive volumes of HTTP requests from multiple sources.

**Detection:**
```rust
// Circuit-based rate limiting
if requests_per_circuit > threshold {
    block_circuit();
}

// Global attack detection
if active_circuits > 100 {
    log_warning("Probable DDoS attack");
}
```

**Mitigations:**

| Layer | Mitigation | Implementation |
|-------|------------|----------------|
| Entry | CAPTCHA gate | Unknown sessions must verify before access |
| Circuit | Per-circuit quotas | 10 req/10sec for unknown, 100 for verified |
| Behavioral | Pattern detection | Rapid requests flagged and demoted |
| Infrastructure | Auto-scaling | Controller spawns additional resources under load |

**Result:** ✅ Attack traffic isolated per-circuit, legitimate users unaffected

**CAPTCHA Fallback Under Load:**

During extreme load, the system uses cascading fallback to ensure users always receive a CAPTCHA challenge:

1. **Primary:** Configured CAPTCHA type (BmpText, Emoji, Direction, etc.)
2. **Fallback:** Lightweight Emoji CAPTCHA (always succeeds)
3. **Failure:** 503 Service Unavailable with retry instructions

**The old "Request Entry" landing page is NEVER shown as a fallback.** This ensures consistent UX even under attack.

```rust
// Gate fallback logic (fortify-gate/src/server.rs)
match create_verification_with_type(captcha_type) {
    Ok(state) => serve_captcha(state),
    Err(_) => {
        // Try lightweight Emoji fallback
        match create_verification_with_type(CaptchaType::Emoji) {
            Ok(state) => serve_captcha(state),
            Err(_) => serve_503_error(), // Never old landing page
        }
    }
}
```

---

### Slow-Loris / Slowdown Attacks

**Attack Description:**
Keeping connections open indefinitely to exhaust server resources.

**Detection:**
```rust
// Timeout monitoring
if connection_age > max_connection_time {
    force_close();
}
```

**Mitigations:**

| Timeout | Value | Purpose |
|---------|-------|---------|
| TCP handshake | 10s | Prevent incomplete connections |
| Header read | 30s | Prevent slow header attacks |
| Full request | 60s | Prevent body slowdowns |
| Idle connection | 90s | Cleanup stale connections |

**Implementation:**
```rust
// fortify-http/src/lib.rs
let server = Server::bind(&addr)
    .http1_header_read_timeout(Duration::from_secs(30))
    .serve(make_service);

// Backend proxy with timeout
let response = tokio::time::timeout(
    Duration::from_secs(60),
    client.request(req)
).await??;
```

**Result:** ✅ Incomplete connections automatically closed

---

## Scraping / Bot Attacks

### Automated Scrapers

**Attack Description:**
Bots crawling site content for data exfiltration.

**Detection:**
```rust
// User-Agent analysis
if user_agent.contains("bot") || user_agent.contains("crawler") {
    flag_violation(SuspiciousUserAgent);
}

// Path enumeration
if sequential_paths >= 5 {
    flag_violation(PathEnumeration);
}
```

**Mitigations:**

1. **CAPTCHA Barrier:** Bots without JS can't solve challenges
2. **User-Agent Detection:** 34+ bot patterns flagged
3. **Behavioral Tracking:** Scraping patterns detected
4. **Progressive Demotion:** 3 violations → SUSPICIOUS → 2 CAPTCHAs

**Result:** ✅ Bot access blocked, real users unaffected

---

### CAPTCHA Solving Services

**Attack Description:**
Using paid human CAPTCHA solvers to bypass verification.

**Detection:**
```rust
// Track session behavior post-verification
if violations_after_verification >= 3 {
    demote_to_suspicious();
}

// Track demotion history
if demotion_count >= 3 {
    mark_as_killed();
}
```

**Mitigations:**

| Defense | Description |
|---------|-------------|
| Behavioral analysis | Detect bot patterns even with valid token |
| Multi-CAPTCHA | Demoted users face 2 harder CAPTCHAs |
| Economic pressure | Each solve costs $ but session gets burned quickly |
| Kill threshold | 3 demotions = permanent burn |

**Cost Analysis:**
```
CAPTCHA solve cost: $0.50 - $2.00
Violations before demotion: 3
Demotions before kill: 3
Total cost per kill: $1.50 - $6.00 × 3 = $4.50 - $18.00

For sustained attack:
- 1,000 sessions killed = $4,500 - $18,000
- Attack becomes economically unfeasible
```

**Result:** ✅ Automated solving unprofitable

---

## Tor-Specific Attacks

### Guard Discovery

**Attack Description:**
Creating many circuits to statistically determine guard nodes, then targeting them.

**Detection:**
```rust
// Vanguards monitors circuit creation patterns
// Logs alerts from vanguards addon
parse_vanguards_alerts(log_line);
```

**Mitigations:**

1. **Vanguards Addon:** Layer 2/3 guard pinning
   ```bash
   VANGUARDS_ENABLED=true
   VANGUARDS_LAYER2_GUARDS=4
   VANGUARDS_LAYER3_GUARDS=8
   ```

2. **Circuit Rotation:** Regular circuit refresh
   ```toml
   VANGUARDS_CIRC_MAX_AGE_HOURS=24
   ```

3. **Attack Alerts:** Vanguards logs suspicious patterns
   ```
   🛡️ Vanguards detected potential guard discovery attempt
   ```

**Result:** ✅ Guard nodes protected from enumeration

---

### Circuit Correlation

**Attack Description:**
Analyzing timing patterns across circuits to correlate users or identify backend.

**Mitigations:**

1. **Multiple Entry Points:** 3-5 active mirrors
2. **Separate Circuits:** Each node has own .onion
3. **Traffic Mixing:** Requests routed through different paths
4. **No IP Logging:** Zero correlation data stored

**Result:** ✅ Circuit correlation infeasible

---

## Web Application Attacks

### Path Traversal

**Attack Description:**
Attempting to access files outside web root (`../../../etc/passwd`).

**Detection:**
```rust
const ATTACK_PATTERNS: &[&str] = &[
    "../", "..\\", "/.env", "/.git",
    "/wp-admin", "/phpmyadmin", "/backup",
];

if path_contains_attack_pattern(&request.path) {
    flag_violation(AttackPathAccess);
}
```

**Mitigations:**

1. **Pattern Detection:** 25+ attack path patterns
2. **High Severity:** 3 points per violation
3. **Immediate Demotion:** 3 violations = SUSPICIOUS tier
4. **Audit Logging:** All attempts logged

**Result:** ✅ Attack attempts blocked and tracked

---

### Directory Scanning

**Attack Description:**
Rapidly accessing many paths to map site structure.

**Detection:**
```rust
// Resource enumeration
if unique_paths_per_minute > 60 {
    flag_violation(ResourceEnumeration);
}

// Sequential scanning (/page1, /page2, ...)
if sequential_path_count >= 5 {
    flag_violation(PathEnumeration);
}
```

**Mitigations:**

1. **Rate Limits:** 60 unique paths/minute threshold
2. **Sequence Detection:** 5+ sequential paths flagged
3. **Progressive Demotion:** Violations tracked and accumulated

**Result:** ✅ Scanning detected and stopped

---

### Form Abuse

**Attack Description:**
Flooding POST endpoints to overwhelm backend or brute-force credentials.

**Detection:**
```rust
// Track POST requests
if post_count_per_minute > 10 {
    flag_violation(FormSubmissionFlood);
}
```

**Mitigations:**

1. **POST Rate Limiting:** 10 submissions/minute
2. **Backend Protection:** Node passes violations to backend
3. **Auto-Demotion:** Flood triggers SUSPICIOUS demotion

**Result:** ✅ Form floods blocked

---

## Infrastructure Attacks

### Mirror Compromise

**Attack Description:**
Identifying and compromising public mirror addresses.

**Detection:**
```rust
// Manual burn trigger (rarely used)
POST /ctrl_xxx/burn-mirror

// Automated burning capability exists but rarely used
if operator_decides_to_burn {
    burn_mirror(mirror_id);
}
```

**Mitigations:**

1. **Burn Capability:** Mirrors CAN be burned if needed
2. **Grace Period:** 24h death page before full destruction (if burned)
3. **Replacement Option:** New mirrors can be created if needed
4. **Multiple Active:** Always maintain 3-5 active mirrors

**Burn Process (when used):**
```
1. Mark mirror as burned (stop accepting new sessions)
2. Serve death page for 24 hours
3. Destroy .onion service
4. Spawn replacement mirror
5. Update discovery mechanisms
```

**Result:** ✅ Compromised mirrors quickly replaced

---

### Node Infection

**Attack Description:**
Routing many malicious sessions through one node to compromise it.

**Detection:**
```rust
// Track session health per node
let bad_session_percentage = 
    bad_sessions / total_sessions;

if bad_session_percentage > 0.6 && total_sessions >= 10 {
    burn_node(node_id);
}
```

**Mitigations:**

1. **Health Monitoring:** Per-node session tracking
2. **Auto-Burn:** 60% bad sessions triggers burn
3. **Circuit Isolation:** Each node has own Tor circuit
4. **Replacement:** New nodes auto-spawned

**Result:** ✅ Infected nodes isolated and replaced

---

## Session Attacks

### Token Forgery

**Attack Description:**
Creating fake session tokens or modifying existing ones.

**Detection:**
```rust
// HMAC verification
let signature = Hmac::<Sha256>::new_from_slice(&secret_key)?
    .chain_update(token_data)
    .finalize()
    .into_bytes();

if signature != provided_signature {
    reject_token();
}
```

**Mitigations:**

1. **HMAC-SHA256:** Cryptographic signing
2. **Server-Side State:** Session data not in token
3. **Expiration:** TTL enforced (default 1 hour)
4. **Secret Rotation:** Keys rotated periodically (planned)

**Result:** ✅ Forged tokens detected and rejected

---

### Session Hijacking

**Attack Description:**
Stealing valid session tokens from legitimate users.

**Mitigations:**

1. **HttpOnly Cookies:** Prevents JS access
2. **Secure Flag:** HTTPS-only (when available)
3. **Short TTL:** 1-hour default expiration
4. **Behavioral Binding:** Unusual patterns trigger re-verification

**Limitation:** Tor makes true session binding difficult (no stable IPs)

**Result:** ⚠️ Partial mitigation (Tor constraints limit binding options)

---

## Operational Security

### Admin Panel Security

**Attack Description:**
Unauthorized access to administrative functions.

**Mitigations:**

1. **Password Authentication:** Required for admin access
2. **Session Timeout:** 24-hour auto-logout
3. **Token-Based API:** Orchestrator requires auth token
4. **Audit Logging:** All admin actions logged

```rust
// Admin authentication
if !is_authenticated(cookie) {
    redirect_to_login();
}

// API token validation
if !validate_auth_token(header) {
    return 401_UNAUTHORIZED;
}
```

**Result:** ✅ Admin functions protected

---

### Logging Security

**Mitigations:**

1. **No PII:** Never log IP addresses, user data
2. **Session IDs Only:** Opaque identifiers
3. **Structured Logging:** tracing framework
4. **Log Rotation:** Prevent disk exhaustion

**Example Safe Log:**
```
✅ Session abc123 promoted to VERIFIED
🚫 Session xyz789 demoted: 3 violations
```

**Result:** ✅ Logs useful but privacy-preserving

---

## Configuration Hardening

### Recommended Settings

```toml
[trust]
session_ttl_seconds = 3600          # Short TTL
max_demotions_before_kill = 3       # Aggressive burn
promotion_threshold = 50            # Earn trust slowly

[behavioral]
violation_type_thresholds = 3       # Low tolerance
max_unique_paths_per_minute = 60    # Detect scanning
max_form_submissions_per_minute = 10 # Prevent floods

[rate_limiting]
unknown_tier_limit = 10             # Strict for unknowns
verified_tier_limit = 100           # Reasonable for verified
trusted_tier_limit = 300            # Generous for trusted

[vanguards]
enabled = true                      # Always enable
layer2_guards = 4                   # Standard
layer3_guards = 8                   # Standard
```

---

*See [threat-model.md](threat-model.md) for attack scenarios and [../02-Core-Concepts/behavioral-analysis.md](../02-Core-Concepts/behavioral-analysis.md) for detection details*
