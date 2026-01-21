# Fortify Documentation

## Quick Links

| Document | Description |
|----------|-------------|
| [Architecture](architecture.md) | System design and component overview |
| [Threat Model](threat-model.md) | Security assumptions and attack scenarios |
| [Trust Levels](trust-levels.md) | The 5-tier trust system explained |
| [Scaling Model](scaling-model.md) | Resource-aware scaling strategy |
| [Hardening](hardening.md) | OS and system hardening procedures |
| [Community Network](community-network.md) | Decentralized discovery system |
| [Testing Guide](TESTING.md) | How to run and write tests |
| [Roadmap](ROADMAP.md) | Future development plans |
| [TUI Deployment Wizard](Dev_Progress/06-PHASE-6.md) | Terminal UI and deployment wizard |

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
