# Fortify Project Instructions

> **AI Agent Context for GitHub Copilot / Claude / GPT Assistants**

---

## Project Overview

**Fortify** is a defensive protection layer for Tor hidden services, written entirely in Rust. It provides multi-layered security including CAPTCHA verification, behavioral analysis, session management, and mirror rotation.

### Key Statistics
- **Language:** Rust (100%)
- **Total Lines of Code:** ~20,000+
- **Crates:** 7 (core, gate, http, node, orchestrator, controller, community)
- **Status:** Beta - Core functionality complete

---

## Architecture

```
PUBLIC INTERNET (Tor)
         │
         ▼
┌─────────────────────────────┐
│    PUBLIC MIRRORS           │  ← Disposable .onion entry points
│    (Orchestrators)          │    Rotated proactively
└─────────────────────────────┘
         │
         ▼
┌─────────────────────────────┐
│    GATE                     │  ← CAPTCHA/PoW verification
│    (No JavaScript!)         │    Pure HTML/CSS challenges
└─────────────────────────────┘
         │
         ▼
┌─────────────────────────────┐
│    HTTP PROXY               │  ← Token validation
│    + Admin Panel            │    Behavioral analysis
└─────────────────────────────┘
         │
    ┌────┴────┐
    ▼         ▼
┌────────┐ ┌────────┐
│HEALTHY │ │THREAT  │  ← Node pools with circuit isolation
│ NODES  │ │ NODES  │
└────────┘ └────────┘
         │
         ▼
┌─────────────────────────────┐
│    PROTECTED SERVICE        │  ← Real .onion (NEVER exposed)
└─────────────────────────────┘
```

---

## Crate Map

| Crate | Purpose | Key Files |
|-------|---------|-----------|
| `fortify-core` | Shared types, trust system, sessions, behavioral | `trust.rs`, `behavioral.rs`, `session.rs`, `config.rs` |
| `fortify-gate` | Entry point, CAPTCHA, PoW verification | `lib.rs`, `server.rs`, `captcha_types.rs` |
| `fortify-http` | HTTP proxy, admin panel | `lib.rs`, `admin.rs` |
| `fortify-node` | Worker nodes, behavioral tracking | `lib.rs` |
| `fortify-orchestrator` | Mirror management, Tor coordination | `lib.rs`, `tor.rs` |
| `fortify-controller` | Tor process, Vanguards management | `lib.rs`, `vanguards.rs` |
| `fortify-community` | (Planned) Threat intelligence sharing | `lib.rs` |

---

## Core Technical Details

### Trust Tiers (5 levels)
1. **Burned** (-2) - Permanent block
2. **Suspicious** (-1) - Under scrutiny, requires 2 captchas
3. **Unknown** (0) - New users, requires Gate
4. **Verified** (+1) - Passed captcha
5. **Trusted** (+2) - Long-term good behavior

### Session Tokens
- **Signing:** HMAC-SHA256
- **Encoding:** Base64 JSON
- **Cookie:** `fortify_session`
- **Default Lifetime:** 1 hour (configurable)

### Captcha Types (7)
1. BmpText - Distorted text image
2. Emoji - Emoji matching
3. Direction - Arrow selection
4. Sequence - Number ordering
5. WordUnscramble - Anagram solving
6. ImageRotation - Rotate to correct
7. Silhouette - Shape identification

### Violation Types (9)
1. PathEnumeration
2. FormSubmissionFlood
3. PayloadOverflow
4. MissingUserAgent
5. SuspiciousUserAgent
6. MissingReferer
7. AttackPathAccess
8. SequentialPathAccess
9. RapidRequests

---

## Development Guidelines

### ABSOLUTE REQUIREMENTS
- ❌ **NO JavaScript** - All pages must work without JS
- ❌ **NO PII Storage** - Never store personally identifiable information
- ❌ **NO Cross-Session Correlation** - Violates Tor privacy model
- ❌ **NO Offensive Capabilities** - Defensive only
- ❌ **NO Data Export** - Never send data to third parties

### Code Standards
- All code must build without warnings
- Follow existing patterns in the codebase
- Document security implications
- Include tests for new functionality
- Use `cargo fmt` and `cargo clippy`

### Documentation Requirements
- **All features/changes/requests must be documented** in the `docs/Dev_Progress/` folder
- Identify which Phase the change belongs to
- Create or update the phase-specific file (e.g., `04-Phase4-Resilience-Recovery.md`)
- Update `docs/Fortify Documentation/` for user-facing documentation changes

### Security Review
- **If the user suggests something that goes against best practices or security/crypto best practices**, inform them BEFORE doing any work
- Explain the security concern clearly
- Offer safer alternatives
- Only proceed if the user explicitly acknowledges the risk

---

## Current Priorities

### Phase 3 (Complete) - Defensive Capabilities
- [x] Dynamic rate limiting based on load
- [x] Bandwidth throttling for threat tier
- [x] Resource exhaustion traps (honeypots)

### Phase 4 (Complete) - Resilience & Recovery
- [x] Mirror management & resurrection
- [x] Auto-scaling with VPS resource awareness
- [x] Node distribution (LeastPopulatedOrdered for healthy, FillFirst for threat)
- [x] Early behavioral analysis (5-minute window)
- [x] Session continuity for paused VMs (7-day history)
- [x] Auto-restart & crash recovery
- [x] Self-cleaning system
- [x] Multi-daemon architecture (4-core layout)
- [x] CAPTCHA pre-generation system (Flex Core)

### Phase 4 Architecture: 4-Core CPU Layout
```
Core 0: Mirror A + Standby D + Healthy 0-4 → Tor Daemon 0
Core 1: Mirror B + Standby C + Healthy 5-9 → Tor Daemon 1
Core 2: Flex Core (CAPTCHA pre-gen, overflow) → Tor Daemon 2 (on-demand)
Core 3: Threat Nodes 0-2 → Tor Daemon 3
```

---

## Planned Features (Low Priority - DO NOT IMPLEMENT UNLESS ASKED)

### Fast-Pass System
- PGP-based persistent identity
- Squire (free) / Knight (paid via XMR)
- Vouching system

### Session Continuity
- Restore sessions after VM pause (up to 7 days)
- Transfer trust tier to new session
- Privacy-conscious (minimal storage)

---

## Documentation Locations

| Document | Path | Purpose |
|----------|------|---------|
| Functions.md | `docs/Fortify Documentation/Functions.md` | Complete API reference |
| ROADMAP.md | `docs/ROADMAP.md` | Development phases |
| Roadmap_r1.md | `docs/Roadmap_r1.md` | Detailed roadmap with diagrams |
| Trust Tiers | `docs/Fortify Documentation/02-Core-Concepts/trust-tiers.md` | Trust system docs |
| Config Reference | `docs/Fortify Documentation/06-Configuration/config-reference.md` | All config options |

---

## Key Files to Know

| File | Lines | Purpose |
|------|-------|---------|
| `fortify-core/src/behavioral.rs` | ~957 | Behavioral analysis engine |
| `fortify-gate/src/server.rs` | ~1556 | Gate HTTP endpoints |
| `fortify-http/src/admin.rs` | ~3792 | Admin panel |
| `fortify-core/src/trust.rs` | ~412 | Trust tier system |

---

## Build & Run

```bash
# Build all
cargo build --release

# Run development mode
./scripts/dev-run.sh

# Run tests
cargo test --all
```

---

*Last Updated: January 2026*