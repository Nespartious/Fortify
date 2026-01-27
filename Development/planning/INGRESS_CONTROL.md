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

## Ingress Control: Comparative Analysis & Performance Ladder

### 1. Nginx (Baseline, Already Near-Optimal)
- Event-driven (epoll/kqueue), zero-copy sendfile, minimal allocations.
- Battle-tested, within 5–10% of theoretical max efficiency for general-purpose proxies.
- Marginal gains possible by disabling unneeded features (logs, gzip, regex, keepalive, etc.), using UNIX sockets, and `reuseport`.
- Diminishing returns: as a generic HTTP proxy, Nginx is the end of the road for most setups.

### 2. HAProxy (Serious Contender)
- Faster connection teardown, more deterministic latency under load.
- Excellent stick-tables for shared state (circuit reputation cache, per-connection state).
- Very fast reject logic, strong at L4/L7 gating hybrids.
- Downsides: Less flexible scripting than Lua Nginx, harder to serve challenge pages, smaller Tor-specific ecosystem.
- Verdict: For "allow/deny/rate/kill" logic, HAProxy can equal or slightly beat Nginx.

### 3. Envoy (Powerful, but Overkill)
- Advanced filters, WASM extensibility, sophisticated rate limiting.
- Cons: Memory heavy, complex, slower cold-path rejects, designed for microservices not onion defense.
- Envoy is best when correctness > raw rejection speed. For Tor, you want the opposite.

### 4. Kernel-Level Tricks (Where Real Gains Appear)
- **iptables/nftables:** Fastest possible drops, zero userspace cost. No Tor/HTTP/session awareness, but great for SYN floods, known-bad patterns, emergency circuit nuking.
- **eBPF/XDP:** Packet rejection before socket creation, near line-rate drops, microsecond decisions. Can enforce packet rate, handshake timing, byte thresholds. Complexity and portability are challenges. Still lacks clean Tor circuit identity, but is used by EndGame-class operators for coarse filtering.
- **Special Emphasis:** Kernel drop is the absolute fastest way to reject traffic, and should be considered as a guardrail for Fortify.

### 5. Custom Rust Ingress (Endgame Move)
- Surpasses Nginx meaningfully: parses only enough HTTP to decide, allocates almost nothing, tracks per-connection micro-state, enforces PoW inline, closes connections before full request parse, integrates directly with Tor control port.
- Enables circuit-aware reputation, difficulty escalation, dynamic challenge shaping, deterministic resource caps.
- **Special Emphasis:** This is the only way to truly surpass Nginx and reach EndGame-class defense. Rust ingress is the "quiet enforcer"—circuit-aware, economic gating, and direct Tor integration.

### 6. Tor-Native Tuning (High Leverage, Often Ignored)
- Tor itself offers knobs: MaxClientCircuitsPending, MaxStreamsPerCircuit, CircuitBuildTimeout, stream idle timeouts, intro point limits.
- These reduce attack surface before Nginx even sees traffic. EndGame-class setups always tune Tor aggressively.

---

## Honest Performance Ladder

- **Kernel drop (iptables/XDP):** Fastest possible
- **Custom Rust ingress:** Endgame tier
- **HAProxy:** Very strong
- **Nginx:** Excellent baseline
- **Envoy:** Heavy, not ideal
- **Backend app:** Too late

---

## Fortify Implementation Roadmap

### Phase 1 (Now, Pragmatic)
- Nginx as ingress
- Lua for verify-or-drop
- Aggressive timeouts
- UNIX socket to Fortify
- No feature creep

### Phase 2 (EndGame Parity)
- Replace Lua logic with Rust ingress
- Nginx optional or removed
- Direct Tor control integration
- Circuit economics enforced before backend

### Phase 3 (Nuclear)
- Optional eBPF guardrail
- Tor-native pressure tuning
- Backend becomes almost irrelevant

---

## Key Takeaways
- If Fortify stays "HTTP proxy + rules," it will never fully match EndGame.
- To reach EndGame-class, Fortify must become a "circuit-aware economic gate that happens to speak HTTP."
- **Special focus for future work:**
  - Kernel-level drops for raw speed
  - Custom Rust ingress for circuit-aware, economic gating
  - HAProxy as a strong alternative to Nginx for certain gating logic

---

**Bottom line:**

> Be ruthless at the door. Nginx is our first and fastest line of defense. But to truly reach EndGame-class, Fortify must evolve: kernel drops for speed, Rust ingress for intelligence, and circuit economics for true onion service resilience.
