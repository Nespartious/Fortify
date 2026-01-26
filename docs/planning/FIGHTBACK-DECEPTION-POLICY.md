# Fortify Fight-Back & Deception Policy (Draft)

Status: Draft — for review  
Purpose: Define safe, legal, and operationally robust deception and defensive "fight-back" controls Fortify may use to increase attacker cost, gather TTPs, and protect real hidden-service backends while preserving Tor user privacy.

Preface — rules of engagement
- Fortify uses only defensive deception and resource-wasting techniques. We do NOT perform active deanonymization, offense, or attribution (no probing-back, DDoS, malware, or traffic confirmation).
- All honeypots, tarpits and deception must be isolated from real backends and avoid collection of PII. Legal counsel must sign off on deployment models in each jurisdiction.
- Principle: Observe first, enforce second. New deception features are introduced in LOG-ONLY mode, then soft-blocks, then (only if safe) hard-blocks.

Contents
1. Definitions
2. High-level policy statements
3. Safe honeypot & tarpit design (constraints & behaviors)
4. Multi-PoW layered defense (mirror → node)
5. Telemetry schema & Grafana panels
6. Rollout plan (canaries + gates)
7. Runbook excerpts (enable/disable, emergency rollback)
8. Forensics, privacy, retention, and legal
9. Appendix: example endpoints & alert rules

---

## 1. Definitions
- Honeypot endpoint: A deliberately planted endpoint attractive to scanners/scrapers. Returns plausible content but is isolated.
- Tarpit: A long-lived, low-bandwidth response designed to waste attacker time while consuming minimal server CPU.
- PoW: Proof-of-Work challenge (hashcash-like) used as an entrance fee.
- Mirror: Public-facing Fortify node/mirror that issues initial PoW/CAPTCHA.
- Node: Reverse-proxy node (node pool) that forwards to protected hidden service; may require secondary PoW.
- LOG-ONLY: Mode where behavior is recorded but no enforcement action taken.

---

## 2. High-level policy statements
- Safety: No active attempts to deanonymize or retaliate against clients.
- Isolation: Honeypots and tarpits run in sandboxed containers/namespaces without network access to production backends or secrets.
- Observability-first: All deception features are enabled in LOG-ONLY on canary nodes for telemetry collection before any enforcement.
- Escalation: Enforcement is tiered — LOG-ONLY → soft-block (revoke session / require reverify) → hard-block (403) — only after acceptable false-positive SLOs.
- Privacy: Do not store raw payloads, raw session IDs, or UA strings. Hash identifiers and redact payloads. Short retention on raw captures (7–14d, isolated).

---

## 3. Safe honeypot & tarpit design

Design goals
- High-signal detection of automation.
- Minimal resource cost to defender.
- No collateral harm to real users or external systems.

Tarpit constraints (MUST)
- Run in isolated process/container with cgroup limits:
  - CPU: ≤ 10% of host 1 vCPU equivalent
  - Memory: ≤ 128–512 MB depending on host
  - Max concurrent connections per tarpit container: configurable (default 100)
- Admission filter:
  - Require a tiny admission check (rate limit or ultra-cheap PoW) before allowing a long-held tarpit connection.
  - Per-circuit token bucket to prevent one circuit from opening many tarpits.
- Resource bounds:
  - Global cap on tarpit slots per node (default 50).
  - Queue backpressure: return 503 / short redirect when queue full.
- Networking:
  - No access to internal backends; read-only fake data only.
  - Outgoing network egress disabled.
- Observability:
  - All events labeled honeypot=true and routed to remote logs (Loki) via Grafana Agent.
  - Track: connection start/end, bytes read/written, duration_ms, challenge_id (if any), session_hash.
- Behavior:
  - Serve slow chunked HTML or streaming "hold your position" content to encourage bot persistence.
  - Keep server CPU low: async writes and sleep intervals.

Honeypot best-practices
- Use low-value but plausible payloads. Never host real credentials or PII.
- Embed honeytokens (unique URLs or ephemeral tokens). Monitor access to tokens as high-confidence signals.
- Keep honeypot footprint minimal and ephemeral. Recreate containers frequently.

---

## 4. Multi‑PoW layered defense (mirror → node)

Rationale
- Multiple sequential PoW steps multiply attacker cost and make stateless cookie-deletion attacks expensive.

Architecture overview (conceptual)
- Mirror: public PoW + CAPTCHA to issue a session token (TrustTier=Verified). Baseline difficulty: Nbits=10.
- Node: reverse-proxy node may require a second PoW (lighter or conditional). Baseline optional: Nbits=8; escalate to 12/16 for suspicious sessions.
- Binding: each PoW is bound to a unique challenge_id and session_id; single-use only.

State rules
- Challenge object:
  - { challenge_id, session_id_hash, challenge_blob, difficulty, created_at, expires_at, used=false }
  - TTL: 300s (configurable)
- Verification flow:
  - On submission, verify nonce vs challenge_blob and difficulty; mark used=true.
  - Reject reused or expired challenges as InvalidProofOfWork.
- Escalation (strike-based):
  - Strike 1 (suspicious): revoke token, issue mirror PoW again.
  - Strike 2 (repeat): node-level difficulty += 2 bits or require 2 sequential PoWs.
  - Strike 3 (confirmed bot): burn session (403) for TTL (e.g., 10m).

Operational guidance
- Node-level PoW is conditional: only enforced if session is untrusted, node load is high, or profiler/dedupe triggers.
- Provide CAPTCHA fallback automatically for clients exceeding solve_time threshold (e.g., >20s).
- Rotate puzzle types occasionally (e.g., different nonce encoding) to make precomputation harder.

Security & replay mitigations
- Single-use + per-session binding prevents reuse across nodes.
- Short TTL and server-side nonce tracking prevents replay.
- Use HMAC-signed challenge IDs to prevent spoofing.

---

## 5. Telemetry schema & Grafana panels

Agent & flow
- Use Grafana Agent on Fortify VPS to remote_write metrics and push logs to Loki. No heavy storage on-host.

Core metrics (Prometheus-style)
- fortify_pow_challenges_issued_total{node, role, difficulty}
- fortify_pow_solved_total{node, role, outcome=success|fail}
- fortify_pow_solve_seconds_bucket/summary
- fortify_profiler_flags_total{flag=cv_low|machine_gun|dedupe}
- fortify_honeypot_hits_total{endpoint, node}
- fortify_tarpit_connections_current{node}
- fortify_session_store_size{node}
- fortify_revocations_total{reason}

Log event fields (structured JSON) — send to Loki
- trace_id: guid
- timestamp: RFC3339
- event_type: challenge_issued | pow_submit | honeypot_hit | tarpit_start | tarpit_end
- session_hash: sha256(session_id)[:12]
- challenge_id
- node_id
- difficulty
- solve_time_ms
- cv_value
- median_iat_ms
- honeypot_endpoint (if applicable)
- note: raw payloads are NOT shipped; dedupe hash only (sha256(payload+uri)[:12])

Suggested Grafana dashboards / panels
- PoW Overview: challenges issued, solved, solve_time p50/p95/p99, failure rate.
- Profiler: CV histogram, machine-gun event rate, flagged sessions over time.
- Honeypot: hits/min by endpoint, unique session_hash count, top endpoints.
- Tarpit: current connections, avg duration_ms, CPU usage by tarpit containers.
- Session-store: active sessions, inserts/sec, memory usage.
- Alerts: PoW failure rate > X%, solve_time p95 > threshold, sudden surge in honeypot hits.

Alert examples
- PoW success rate < 95% (SLO breach)
- p95 solve_time_ms > 5000ms
- honeypot_hits_per_min > baseline * 5
- tarpit_connections_current > cap * 0.8

---

## 6. Rollout plan (safe, observability-first)

Phases
1. POLICY: finalize legal/privacy sign-off.
2. TELEMETRY: implement agent and dashboards on canary node(s).
3. HONEYPOTS/TARPITS: deploy honeypots and tarpit containers in LOG-ONLY mode (canary).
4. PoW MIRROR re-enable: enable PoW logging (no enforcement) on mirror canary group.
5. MONITORING/TUNE: collect 2–4 weeks telemetry, adjust thresholds.
6. NODE-POW SAMPLE: enable node-level PoW for sample % (1–5%) in log-only → soft-block.
7. SOFT ENFORCEMENT: enable soft-blocking (revoke session / require reverify) in staged fashion.
8. HARD ENFORCEMENT: enable hard-block rules only after SLOs and false-positive checks passed.
9. FULL ROLLOUT: 100% with ongoing monitoring.

Gates & Metrics for progression
- PoW solve_time p95 below target on human sample.
- False-positive rate < SLO (e.g., < 1%) across canary.
- Session-store and node CPU within capacity.

---

## 7. Runbook excerpts (enable/disable, emergency rollback)

Enable PoW (canary)
1. Set config: POW_MODE=LOG_ONLY on canary nodes.
2. Deploy Grafana Agent configs; verify PoW logging appears.
3. After baseline period, switch POW_MODE=SOFT_BLOCK for canary.

Disable PoW (emergency)
- Global emergency toggle: set POW_MODE=DISABLED via orchestrator (TUI/Env).
- Immediately revoke/clear pending verification states if memory pressure present.
- Steps must be executable within 2 minutes.

Tarpit emergency stop
- Command: scale tarpit container pool to 0 or set MAX_TARPIT_SLOTS=0.
- Fallback: block tarpit endpoint via ingress rule.

Investigation workflow
- Alert triggers -> open incident ticket -> analyst pulls trace_id -> inspect Honeypot/Loki entries (session_hash, cv, solve_time) -> decide escalation (watchlist, soft-block, burn) -> document.

---

## 8. Forensics, privacy, retention & legal
- Raw honeypot captures: store in isolated encrypted storage; retention default 7 days (extend only with approval).
- Aggregated telemetry (metrics & hashed IDs): retention 30–90 days.
- Access control: RBAC on Grafana/Loki; analyst access logged and audited.
- Legal: coordinate with counsel; document jurisdiction-specific constraints.

---

## 9. Appendix — example endpoints & alert rules

Suggested honeypot endpoints
- `/.env.backup` — default 200, honeypot=true
- `/admin_old/login` — default 200, high-alert
- `/api/v1/test-data?token=<unique_honeytoken>` — detect exfil reuse
- `/download/secret-archive.tar.gz` — return fake archive (small) and log

Alert rules (examples)
- `sum(rate(fortify_honeypot_hits_total[5m])) > 100` -> HIGH: investigate scan surge
- `increase(fortify_profiler_flags_total{flag="machine_gun"}[10m]) > 20` -> MEDIUM
- `fortify_pow_solved_total{outcome="fail"} / fortify_pow_challenges_issued_total > 0.1` -> REVIEW

---

End of document — ready for incorporation into Fortify docs.
- Next step: if approved, I will generate a final "FightBackPolicy.md" suitable for repository commit with operator checklist and Grafana Agent config templates.
