# Tor Research & Practical Defenses Implementation Roadmap

**Status**: 📋 PENDING REVIEW  
**Created**: 2025-01-22  
**Type**: Production-Ready Implementation Plan  
**Review Required**: Yes — scheduled for team dissection

---

## Overview

Below is a prioritized, production-ready implementation roadmap for integrating the recent Tor research and practical defenses into Fortify. No code — only milestones, tasks, owners, success criteria, and rollout/test guidance.

This document includes the Grafana Agent + remote telemetry flow and all monitoring-related work as first-class items (design, deploy, secure, tune).

---

## High-Level Change

Telemetry and monitoring are now explicit cross-cutting concerns:
- Telemetry design (schema + privacy)
- Agent-based shipping design (Grafana Agent / promtail)
- Remote storage (Prometheus remote_write, Loki, Tempo or Grafana Cloud)
- Dashboards & alerts (Grafana)
- Operational hardening (TLS, tokens, retention, RBAC)

The telemetry/agent flow is treated as **mandatory before any enforcement changes** (PoW/profiler enforcement must be backed by canary telemetry).

---

## Updated Prioritized Roadmap (Short)

1. HMAC secret enforcement (policy)
2. Telemetry & monitoring design (expanded)
3. Agent-based telemetry shipping (Grafana Agent / promtail)
4. PoW re-enable plan (log-only canary)
5. Session-store concurrency migration (DashMap)
6. Monotonic-timing profiler (log-only)
7. Canary telemetry collection and tuning
8. Safe enforcement (soft blocks, ramp)
9. Dedupe/fingerprint engine
10. App-layer obfuscation & padding options
11. ML anomaly pipeline (optional)
12. Arti integration plan (optional)
13. Full rollout & ongoing ops

---

## Detailed Milestones

### 1. Policy & Safety Gate — HMAC Secret Enforcement

**Objective**: Ensure startup fails if HMAC secret missing.

| Attribute | Value |
|-----------|-------|
| Owner | Ops/Security |
| Effort | 1–2 days |
| Success Criteria | Fortify refuses to start without valid HMAC secret |

---

### 2. Telemetry & Monitoring Design (Critical — Expanded)

**Objective**: Define the telemetry schema, privacy rules, storage targets, and dashboard requirements before any enforcement rollouts.

#### Tasks

- Define metrics (names + units), logs (structured JSON events), and traces to collect.

**Required Metrics (examples)**:
- `fortify_pow_challenges_total`
- `fortify_pow_solved_total`
- `fortify_pow_solve_seconds{quantile}`
- `fortify_profiler_flags_total{flag}`
- `fortify_dedupe_events_total`
- `fortify_sessions_active`
- `fortify_session_store_ops_total`

**Log Event Schema**:
```json
{
  "trace_id": "...",
  "session_id_hash": "...",
  "event_type": "...",
  "challenge_id": "...",
  "pow_nonce_result": "...",
  "solve_time_ms": 0,
  "cv": 0.0,
  "median_iat_ms": 0,
  "dedupe_hash": "...",
  "reason": "..."
}
```

**Redaction Rules**: No raw payloads, session_id hashed (sha256 prefix).

- Decide retention (e.g., detailed logs 7–14d, metrics 30–90d).
- Choose remote backend: Grafana Cloud (managed) or self-hosted Prometheus+Cortex/Thanos/Mimir + Loki + Tempo.

| Attribute | Value |
|-----------|-------|
| Owner | Dev + Observability |
| Effort | 3–5 days |
| Success Criteria | Telemetry schema documented; data-retention and privacy policy approved |

---

### 3. Agent-Based Telemetry Shipping (Grafana Agent / Promtail)

**Objective**: Run a lightweight agent on each Fortify VPS to forward metrics/logs/traces to remote backends; avoid running heavy stores on the Fortify host.

#### Tasks

- Select agent: Grafana Agent (metrics + logs + traces) or promtail + node_exporter + otelcol.
- Configure agent to:
  - Expose /metrics locally for scrape by remote system OR use remote_write from agent.
  - Tail Fortify logs (structured JSON) and send to Loki endpoint.
  - Send traces (OTLP) to Tempo endpoint (optional).
  - TLS + token-based auth to remote endpoints.
  - Disk buffering for network outages; cap buffer size.
- Security: store agent tokens in vault; use mTLS if supported.
- Resource limits: cgroup/container caps (e.g., 64–256MB RAM, 0.1–0.5 CPU).

| Attribute | Value |
|-----------|-------|
| Owner | Observability + Ops |
| Effort | 2–4 days |
| Success Criteria | Agent deployed to canary host; metrics/logs appear in remote backend; minimal local resource usage demonstrated |

---

### 4. PoW Re-Enable Plan — Log-Only Canary

**Objective**: Re-enable PoW verification server-side but treat results as telemetry only on canary fleet.

#### Tasks

- Define Nbits baseline, TTL, single-use semantics, UX copy.
- Gate logs PoW events to remote backend via agent.

| Attribute | Value |
|-----------|-------|
| Owner | Gate dev + UX |
| Effort | 3–5 days |
| Success Criteria | PoW events and solve times visible in Grafana dashboards from canary nodes |

---

### 5. Session Store Concurrency Migration (DashMap)

**Objective**: Replace global lock map with DashMap.

- Bench and deploy to canary after telemetry available.

| Attribute | Value |
|-----------|-------|
| Owner | Core dev |
| Effort | 1–2 weeks |
| Success Criteria | Session store operations visible in Grafana; no lock contention under load |

---

### 6. Monotonic-Timing Profiler & Buffer

**Objective**: Use `Instant` and circular buffer, compute CV/IAT, emit log-only events to the telemetry pipeline.

| Attribute | Value |
|-----------|-------|
| Owner | Behavioral dev |
| Effort | 3–7 days |
| Success Criteria | CV/IAT metrics available in Grafana |

---

### 7. Canary Telemetry Collection & Tuning (Observability-First)

**Objective**: Collect 2–4 weeks of telemetry from canary nodes and tune thresholds.

#### Tasks

- Create dashboards:
  - PoW solve-time histogram/p95
  - Profiler CV distribution
  - Dedupe events
  - Session-store metrics
  - CPU usage per node
- Define alerts:
  - PoW success rate drop
  - p95 solve_time exceed
  - Session-store memory growth
- Tune thresholds and SLOs (false-positive budget).

| Attribute | Value |
|-----------|-------|
| Owner | Security + Observability + Data |
| Effort | 2–4 weeks |
| Success Criteria | Baselines set, alerts stable, thresholds chosen |

---

### 8. Safe Enforcement: Soft Blocks & Progressive Friction

**Objective**: Implement strike-based soft blocks; escalation rules driven by telemetry.

- Use Grafana alerts to surface unusual block patterns.

| Attribute | Value |
|-----------|-------|
| Owner | Gate dev |
| Effort | 1–2 weeks |
| Success Criteria | Soft blocks enforced with telemetry-backed rollback capability |

---

### 9. Dedupe / Content Fingerprint Engine

**Objective**: Per-session payload hash dedupe with TTL.

- Log events to Loki; add dashboard panel.

| Attribute | Value |
|-----------|-------|
| Owner | Behavioral dev |
| Effort | 3–7 days |
| Success Criteria | Dedupe events visible in Grafana; false positives < threshold |

---

### 10. App-Level Obfuscation & Padding Options

**Objective**: App-layer randomization and optional WTF-PAD.

- Monitor bandwidth and latency via Grafana (panels: bandwidth_in/out, latency).

| Attribute | Value |
|-----------|-------|
| Owner | Frontend/Gate dev |
| Effort | 2–4 weeks |
| Success Criteria | Obfuscation options configurable; overhead measured and acceptable |

---

### 11. ML-Driven Anomaly Pipeline (Optional)

**Objective**: Use collected telemetry to train detection models (kept behind soft-block gate).

#### Tasks

- Export labeled telemetry (log-only period).
- Build simple scoring model; expose scoring metric to Grafana.

| Attribute | Value |
|-----------|-------|
| Owner | Data/ML |
| Effort | 4–8 weeks |
| Success Criteria | Model scoring visible in Grafana; false-positive rate within budget |

---

### 12. Arti Integration Plan (Optional)

**Objective**: Evaluate and prototype Arti-based components.

- Monitor via Grafana Agent during PoC.

| Attribute | Value |
|-----------|-------|
| Owner | Infra/Core |
| Effort | 2–6 weeks |
| Success Criteria | Arti PoC demonstrates feasibility; decision documented |

---

### 13. Full Rollout & Ongoing Ops

**Objective**: Increase coverage in stages (5–10% → 30% → 60% → 100%), with monitoring checks and rollback toggles at each step.

**Telemetry checkpoints at each expansion**:
- PoW success rate
- False positives
- solve_time p95
- CPU use
- Session-store memory

| Attribute | Value |
|-----------|-------|
| Owner | Ops + Security |
| Effort | 2–4 weeks for staged rollout |
| Success Criteria | 100% rollout with stable metrics and no regressions |

---

## Telemetry & Grafana Specifics (Practical Guidance)

### Agent Choice

**Grafana Agent recommended** (metrics + logs + traces) to remote Grafana Cloud or self-hosted storage.

### Scrape & Retention

- `scrape_interval`: 15–30s (30s default for canary)
- Metric retention: 30–90 days (aggregate long-term)
- Logs: 7–14 days detailed

### Cardinality Rules

⚠️ **DO NOT use per-session or per-request labels.**

Use labels like:
- `trust_tier`
- `node_role`
- `region`

### Dashboards to Create

| Dashboard | Panels |
|-----------|--------|
| **PoW** | Challenges issued, solved, solve_time histogram (p50/p95/p99) |
| **Profiler** | CV distribution, median IAT, flagged sessions per minute |
| **Dedupe** | Dedupe events per minute, top URIs causing dedupe |
| **Session Store** | Active sessions, inserts/deletes/sec, memory usage |
| **System** | CPU, memory, disk for Fortify node and agent |

### Alerts

| Alert | Condition |
|-------|-----------|
| PoW success rate | < SLO (e.g., <95%) |
| p95 solve_time | > threshold (tunable) |
| Profiler flags | Sudden increase |
| Dedupe events | Sudden increase |
| Session-store growth | > 2x baseline |

### Security

- TLS + token-based auth to remote endpoints
- Local agent tokens in vault; use short-lived tokens if supported
- Redact or hash sensitive fields before shipping logs

### Resource Sizing

| Component | Resources |
|-----------|-----------|
| Canary node (Fortify + Agent) | 1–2 vCPU, 2–4 GB RAM; agent limited to ~64–256MB |
| Remote monitoring host | Depends on cardinality; start with 2 vCPU/4GB for small fleets or use Grafana Cloud |

---

## Testing & Rollback

1. Validate agent forwarding on canary; run load tests to measure bandwidth/ingest.
2. Test access controls for Grafana UI (SSO/2FA).
3. **Rollback plan**:
   - Runtime toggle to set agent to "local-only" or change remote_write to blackhole
   - Emergency flag to revert PoW/profiler to LOG-ONLY

---

## Deliverables from This Integrated Plan

- [ ] Documented telemetry schema and privacy rules
- [ ] Grafana Agent config templates and promtail examples
- [ ] Pre-built Grafana dashboards and alert rules for Fortify metrics
- [ ] Canary deployment runbook (how to enable/disable)
- [ ] Full staged rollout plan tied to telemetry gates

---

## Review Notes

*This section will be populated during the scheduled review session.*

### Questions to Address

1. Is the scope appropriate for current resources?
2. Which milestones are critical vs. nice-to-have?
3. Timeline estimates — realistic?
4. Grafana Cloud vs. self-hosted decision criteria?
5. Integration with existing Sprint work?

### Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| TBD | | |

---

*Document ready for team review and dissection.*
