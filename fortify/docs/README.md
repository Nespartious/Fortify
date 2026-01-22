# Fortify Documentation

**Last Updated:** January 22, 2026

---

## Documentation Structure

| Folder | Purpose |
|--------|---------|
| [planning/](planning/) | Project planning, task aggregation, status snapshots |
| [Dev_Progress/](Dev_Progress/) | Sprint guides for execution |
| [Fortify Documentation/](Fortify%20Documentation/) | Final user/operator documentation |
| [research/](research/) | Long-form research and analysis |

---

## Current Status

See [planning/PLANNING-OVERVIEW.md](planning/PLANNING-OVERVIEW.md) for current sprint queue.

| Area | Status |
|------|--------|
| Core Protection | ✅ Production Ready |
| Beta Blockers | ⬜ 2 remaining |
| TUI Wizard | 🟡 40% complete |
| CI/CD | ⚠️ Needs fixes |

---

## Quick Links

### Planning & Progress
| Document | Description |
|----------|-------------|
| [Planning Overview](planning/PLANNING-OVERVIEW.md) | Current sprint queue and workflow |
| [Master Status](planning/MASTER-STATUS-2026-01-22.md) | Complete project status |

### Active Sprints (Dev_Progress)
| Sprint | Priority | Est. Time |
|--------|----------|-----------|
| [Timeout Strategy](Dev_Progress/01-TIMEOUT-STRATEGY-SPRINT.md) | 🔴 CRITICAL | 2-3 days |
| [Panic Audit](Dev_Progress/02-PANIC-AUDIT-SPRINT.md) | 🔴 CRITICAL | 3-5 days |
| [CI Quality](Dev_Progress/03-CI-QUALITY-SPRINT.md) | 🟡 MEDIUM | 1-2 days |
| [TUI Completion](Dev_Progress/04-TUI-COMPLETION-SPRINT.md) | 🟡 MEDIUM | 3-5 days |
| [Clippy Fixes](Dev_Progress/CLIPPY-SPRINT.md) | 🟡 MEDIUM | 3-4 hours |

### Reference Documentation
| Document | Description |
|----------|-------------|
| [AUTHENTICATION.md](AUTHENTICATION.md) | Admin authentication system |
| [RATE_LIMITING.md](RATE_LIMITING.md) | Circuit-based rate limiting |
| [ROADMAP.md](ROADMAP.md) | Full feature roadmap (Phases 1-9) |

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        CONTROLLER                                │
│                   (Resource Management)                          │
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
        ┌──────────┐   ┌──────────┐   ┌──────────┐
        │ MIRROR 1 │   │ MIRROR 2 │   │ MIRROR N │   ← Public Entry
        │ .onion   │   │ .onion   │   │ .onion   │
        └────┬─────┘   └────┬─────┘   └────┬─────┘
             │              │              │
             └──────────────┼──────────────┘
                            ▼
        ┌─────────────────────────────────────────┐
        │            ORCHESTRATOR                  │
        │     (Session Classification)             │
        │     /Fortify → Admin Panel              │
        └─────────────────────────────────────────┘
                            │
                ┌───────────┴───────────┐
                ▼                       ▼
        ┌─────────────┐         ┌─────────────┐
        │    GATE     │         │    HTTP     │
        │  (Captcha)  │         │   (Proxy)   │
        │  Port 8081  │         │  Port 8082  │
        └─────────────┘         └─────────────┘
                │                       │
                │           ┌───────────┴───────────┐
                │           ▼                       ▼
                │   ┌─────────────┐         ┌─────────────┐
                │   │HEALTHY NODE │         │ THREAT NODE │
                │   │ (Verified)  │         │(Suspicious) │
                │   │  Port 9100  │         │  Port 9200  │
                │   └──────┬──────┘         └──────┬──────┘
                │          │                       │
                │          └───────────┬───────────┘
                │                      ▼
                │              ┌─────────────┐
                └─────────────►│   BACKEND   │
                               │  (Private)  │
                               │  .onion     │
                               └─────────────┘
```

## Traffic Flow

### New User (Unknown Tier)
1. User connects to public mirror `.onion`
2. Orchestrator sees no session token → routes to **Gate**
3. Gate presents captcha challenge
4. User solves captcha → receives session token (Verified tier)
5. Subsequent requests route through **HTTP Proxy** → **Healthy Node** → **Backend**

### Demoted User (Suspicious/Threat Tier)
1. User triggers violation (rate limit, suspicious pattern)
2. Node demotes user tier
3. User sees friendly "slow down" message
4. Redirected to Gate for re-verification
5. After captcha, tier resets based on history

### Burned User
1. Too many violations → permanent burn
2. Session deleted
3. Any new sessions from same patterns get immediate burn
4. Must wait for burn TTL to expire

## Configuration

Main config: `config/fortify.example.toml`

```toml
[server]
controller_addr = "127.0.0.1:8080"
gate_addr = "127.0.0.1:8081"
proxy_addr = "127.0.0.1:8082"

[trust]
session_ttl_seconds = 3600
max_violations_before_burn = 5
verification_timeout_seconds = 300

[scaling]
max_orchestrators = 5
max_nodes_per_tier = 3
cpu_threshold_spawn = 80
cpu_threshold_despawn = 30
```

## Security Invariants

1. **Backend Never Exposed**: Real `.onion` address is never in public traffic
2. **No JavaScript**: All challenges are server-side
3. **Fail Closed**: Unknown states route to Gate, not backend
4. **Tiered Access**: Higher trust = more resources, lower latency
5. **Burn Recovery**: Even burned sessions eventually expire

## Development

```bash
# Build
cargo build --workspace

# Test
cargo test --all

# Run development instance (wipes state)
./scripts/dev-run.sh --wipe

# View admin panel
# Browse to http://127.0.0.1:8082/Fortify/admin (after verification)
```

## Original Specifications

Design documents archived in [specs/](specs/):
- [fortify-master.md](specs/fortify-master.md) - Original architecture
- [defensive-mechanisms.md](specs/defensive-mechanisms.md) - Security specifications
- [development-stack.md](specs/development-stack.md) - Technology choices
