# Ingress Control: Fast Threat Rejection with Nginx

## Summary

The core principle: **Kill connections we deem are threats FAST.** Like hiring in the workplace—no one is good at hiring, but you must be good at firing. For Tor hidden services, this means using Nginx as a high-performance ingress filter to aggressively drop suspicious or abusive connections before they reach our Rust-based Fortify service.

## Why Nginx?
- Nginx is proven, fast, and highly configurable for connection management.
- It can enforce rate limits, timeouts, and custom Lua/JS logic at the edge.
- It is the "bouncer"—Fortify is the "manager".

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

---

**Bottom line:**

> Be ruthless at the door. Nginx is our first and fastest line of defense. The goal is not perfect hiring, but perfect firing—drop threats before they cost us resources.
