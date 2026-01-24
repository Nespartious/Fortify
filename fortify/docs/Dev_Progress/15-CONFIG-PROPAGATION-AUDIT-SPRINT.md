# Sprint 15: Configuration Propagation Audit & Fix

## Status: IN PROGRESS

## Objective
Audit and fix the configuration propagation system to ensure all TUI settings are correctly wired through the system.

---

## Phase 1: Audit Complete ✅

### Key Findings

#### Q1 ANSWER: Which Config Model is Active?

**BehaviorConfig (from fortify-core) is the ACTIVE system.**

Evidence from HTTP Proxy (`fortify-http/src/lib.rs`):
```rust
let behavior_config = admin_state.get_behavior_config();
// ...
if behavior_config.should_demote_to_threat(&stats) {
    // demote session
}
let max_demotions = behavior_config.max_demotions_before_kill;
```

**ThresholdConfig (in TUI) is ORPHANED - never connected to anything.**

**Conclusion:** TUI's ThresholdConfig needs to either:
- Be removed and replaced with BehaviorConfig fields in TUI
- Or wired through to HTTP proxy's BehaviorConfig

The BehaviorConfig from fortify-core is the correct model per documentation.

---

#### Q2 ANSWER: Mirrors vs Orchestrators

From documentation (`01-Architecture/overview.md`):

```
ORCHESTRATOR (Mirror Mgr)
├── Create Tor hidden services (ADD_ONION/file-based)
├── Maintain minimum active mirrors
├── Maintain standby mirrors (paused, ready)
└── Delete burned mirrors
```

**Relationship:**
- **Orchestrators** = Service managers that CREATE and MANAGE mirrors
- **Mirrors** = The actual .onion addresses that users connect to
- One Orchestrator can manage MULTIPLE mirrors

**TUI MirrorConfig controls:**
- How many mirrors to create (min/max/standby)
- Rotation timing
- Burn behavior

These should flow through Controller → Orchestrator. Currently they DON'T.

---

#### Active Config Summary

| Config Area | TUI Has | Actually Used By | Status |
|-------------|---------|------------------|--------|
| Branding | BrandingConfig (5 fields) | Gate via env vars | ✅ Working |
| CAPTCHA Pool | CaptchaConfig (6 fields) | Controller → Orchestrator | ✅ Partial |
| CAPTCHA Types | CaptchaConfig (8 fields) | Gate (hardcoded defaults) | 🔴 NOT WIRED |
| Thresholds | ThresholdConfig (10 fields) | HTTP uses BehaviorConfig | 🔴 ORPHANED |
| Network | NetworkConfig (9 fields) | Controller (4 fields only) | 🔴 PARTIAL |
| Mirrors | MirrorConfig (8 fields) | Controller (hardcoded) | 🔴 NOT WIRED |
| Vanity | VanityConfig (6 fields) | Controller (3 fields only) | ⚠️ PARTIAL |
| Traffic Tier | TrafficTier | Affects CAPTCHA pool only | ⚠️ PARTIAL |

---

## Phase 2: Decisions Made

### D1: Hot-Reload Deferred
**Decision:** Disable settings modification during deployment. Implement hot-reload later.
- Need to disable the settings screen when system is deployed

### D2: Captcha Template Refactor = Separate Sprint
**Decision:** Converting 6 captcha types to use template engine is Sprint 16.
- For now, captcha pages (except BmpText) will show "FORTIFY" hardcoded

### D3: ThresholdConfig → BehaviorConfig Alignment
**Decision:** TUI's ThresholdConfig should be replaced/aligned with BehaviorConfig.
- BehaviorConfig is the active system (documented, implemented)
- ThresholdConfig fields don't exist anywhere else

---

## Phase 3: Prioritized Fix Plan

### Priority 0: Immediate Fixes (This Sprint)

| Task | Description | Effort | Status |
|------|-------------|--------|--------|
| P0.1 | Disable settings screen during deployment | Low | ✅ DONE |
| P0.2 | Fix HTTP proxy `serve_killed_session_page()` to use `from_env()` | Low | ✅ DONE |

**P0.1 Implementation Details:**
- Added `is_enabled(is_running: bool)` method to `MenuItem` in `app.rs`
- Added `disabled_hint()` method to explain why items are disabled
- Updated `ui/home.rs` to gray out disabled menu items with hint text
- Updated `handle_menu_key()` to block Enter and hotkeys for disabled items
- Removed 'M' hotkey from running view (only 'V' for view-only allowed)

**P0.2 Implementation Details:**
- Changed `serve_killed_session_page()` to use `BrandingVars::from_env()`
- Added branding env vars to `proxy_env()` in Controller

### Priority 1: Wire Missing CAPTCHA Settings

| Task | Description | Effort | Status |
|------|-------------|--------|--------|
| P1.1 | Wire `gate_captcha_type` TUI → Controller → Gate | Medium | ✅ DONE |
| P1.2 | Wire `threat_captcha_type` TUI → Controller → Gate | Medium | ✅ DONE |
| P1.3 | Wire `timeout_seconds` (CAPTCHA) TUI → Controller → Gate | Low | ✅ DONE |
| P1.4 | Wire `difficulty` TUI → Controller → Gate | Low | ✅ DONE |

**P1 Implementation Details:**
- Added env vars in TUI deployment.rs: `CAPTCHA_GATE_TYPE`, `CAPTCHA_THREAT_TYPE`, `CAPTCHA_THREAT_ENABLED`, `CAPTCHA_DIFFICULTY`, `CAPTCHA_TIMEOUT`
- Added fields to Controller config.rs and from_env() parsing
- Added forwarding in Controller lib.rs `gate_env()`
- Gate main.rs reads env vars, parses CaptchaType, and applies via `update_captcha_config()`

### Priority 2: Wire Network Settings

| Task | Description | Effort |
|------|-------------|--------|
| P2.1 | Wire `gate_bind` TUI → Controller | Low |
| P2.2 | Wire `http_bind` TUI → Controller | Low |
| P2.3 | Wire `vanguards_*` settings TUI → Controller | Low |

### Priority 3: Resolve Config Mismatch (Design Required)

| Task | Description | Effort |
|------|-------------|--------|
| P3.1 | Design: Align TUI ThresholdConfig with BehaviorConfig | Design |
| P3.2 | Implement TUI behavioral settings UI | High |
| P3.3 | Wire behavioral settings TUI → Controller → HTTP | High |

### Priority 4: Wire Mirror Settings

| Task | Description | Effort |
|------|-------------|--------|
| P4.1 | Wire `min_mirrors`/`max_mirrors` TUI → Controller → Orchestrator | Medium |
| P4.2 | Wire rotation/burn settings TUI → Controller → Orchestrator | Medium |

### Deferred to Sprint 16

| Task | Description |
|------|-------------|
| D1 | Convert 6 captcha types to use template engine |
| D2 | Hot-reload capability for settings |

---

## Implementation Notes

### Branding Flow (Working Reference)
```
TUI deployment.toml
    ↓ [deployment.rs: .env("BRANDING_*")]
Controller process
    ↓ [config.rs: from_env()]
    ↓ [lib.rs: gate_env()]
Gate process
    ↓ [main.rs: BrandingVars::from_env()]
gate.html rendered with branding
```

### Pattern to Apply for Other Settings
1. Add env var in TUI deployment.rs when spawning Controller
2. Add field to ControllerConfig struct
3. Add from_env() parsing in Controller config.rs
4. Add to appropriate *_env() function in Controller lib.rs
5. Read from env in target service's main.rs

---

## Files Modified This Sprint

- `docs/Dev_Progress/15-CONFIG-PROPAGATION-AUDIT-SPRINT.md` (this file)
- `docs/planning/CONFIG-PROPAGATION-ARCHITECTURE.md` (architecture documentation)
- `crates/fortify-tui/src/app.rs` (MenuItem enable/disable logic)
- `crates/fortify-tui/src/ui/home.rs` (grayed out menu items UI)
- `crates/fortify-http/src/lib.rs` (branding for killed session page)
- `crates/fortify-controller/src/lib.rs` (branding env vars to proxy)

---

## Testing Checklist

- [ ] Deploy with TUI, verify settings screen disabled during deployment
- [ ] Verify branding still works on gate.html
- [ ] Verify captcha page branding (BmpText only - others deferred)
- [ ] Verify killed session page uses branding from env

---

## Success Criteria

1. Settings cannot be modified while deployed
2. All P0 and P1 fixes implemented and working
3. Clear documentation of what's wired vs what's not
4. Sprint 16 created for captcha template refactor
