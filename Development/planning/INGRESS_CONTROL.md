# Ingress Control: Fast Threat Rejection with Nginx

## Summary

The core principle: **Kill connections we deem are threats FAST.** Like hiring in the workplace—no one is good at hiring, but you must be good at firing. For Tor hidden services, this means using Nginx as a high-performance ingress filter to aggressively drop suspicious or abusive connections before they reach our Rust-based Fortify service.

---

## Strategy Report: Elevating Fortify to "EndGame-Class" Defense

### Tor Browser Safest Mode: The Constraint
- No JavaScript: No client-side Proof-of-Work, no advanced browser fingerprinting.
- Only raw protocol mechanics and human interaction are available for defense.

### 1. What is Nginx? (The "Bouncer")
- Nginx is the first line of defense, like a bouncer outside a nightclub.
- Handles 10,000+ connections with negligible CPU, dropping threats before they reach the backend.
- If the backend (Fortify) acts as the greeter, attackers already consume resources before being rejected.

### 2. EndGame V3 Audit: "Verify or Drop"
- EndGame V3 uses Lua scripts in Nginx (access_by_lua phase) to intercept requests before they hit the backend.
- Logic:
  - If no valid session cookie, and not a captcha request: `ngx.exit(444)` (immediate connection close, no response).
  - If valid, allow to backend.
- This is mathematically optimal for Tor Layer 7 defense: minimal wait, minimal resource use.
- Weakness: EndGame only blocks HTTP, not the Tor circuit—attackers can retry on the same circuit.

### 3. Fortify v2 Improvement Plan: "Verify or Kill"
- **Move all ingress logic to Nginx.**
  - Include `nginx.conf` and `fortify.lua` in the repo.
  - Use `libnginx-mod-http-lua` or OpenResty.
- **State Machine:**
  - State 0 (Unknown): No Fortify Token → 302 Redirect to /challenge.
  - State 1 (Challenge): Serve static HTML form with unique image.
  - State 2 (Submission): If valid, set token and redirect to /. If invalid, drop (444).
- **Active Circuit Defense:**
  - On abuse or repeated failure, Lua script signals Tor Control Port to `CLOSE_CIRCUIT $CircuitID`.
  - Forces attacker to rebuild circuit (expensive, slow), imposing a real cost.

### 4. Implementation Blueprint
- **A. Nginx Config (`nginx.conf`)**
- **B. Lua Logic (`fortify_access.lua`)**
- **C. Active Defense (`active_kill.lua`)**
  - Call this when a user spams or fails captchas repeatedly.

---

## Strategic Goals
- **Immediate threat rejection:** Use Nginx to drop connections that violate basic rules (rate, headers, timeouts, etc.)
- **Minimize resource waste:** Prevent abusive clients from consuming Tor circuit slots or backend CPU.
- **Simple, auditable rules:** Start with basic heuristics, then iterate.

## Implementation Strategy
1. **Nginx as the Ingress Layer:**
   - All inbound Tor traffic hits Nginx first.
   - Nginx applies connection limits, timeouts, and basic filtering.
   - Only "clean" traffic is proxied to the Fortify Rust service.
2. **Rule Set Examples:**
   - Limit connections per IP (with Tor, use circuit fingerprinting if possible).
   - Aggressive timeouts (e.g., drop slow POSTs, idle connections).
   - Block known bad user-agents or malformed requests.
   - Use Lua/JS for advanced filtering if needed.
3. **Integration:**
   - Fortify runs behind Nginx (localhost or unix socket).
   - Nginx handles TLS (if needed) and all public-facing ports.
   - All logging and monitoring starts at Nginx.

## Open Questions
- How to fingerprint Tor circuits for per-client limits?
- What is the minimal rule set to start with?
- How to pass threat intelligence from Fortify back to Nginx (dynamic blocking)?

## Next Steps
- Prototype a minimal Nginx config that proxies to Fortify and drops slow/abusive connections.
- Document and tune rules iteratively.
- Explore Lua/JS scripting for smarter filtering.
- Implement the Circuit Killer Lua script for active defense.

---

**Bottom line:**

> Be ruthless at the door. Nginx is our first and fastest line of defense. The goal is not perfect hiring, but perfect firing—drop threats before they cost us resources. Fortify v2 must go beyond EndGame by actively killing abusive Tor circuits, not just HTTP requests.
