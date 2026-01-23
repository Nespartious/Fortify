# Sprint 15: Branding & Configuration Propagation

**Sprint ID:** BETA-015  
**Priority:** 🔴 HIGH (Core Functionality)  
**Estimated Effort:** 3-4 days  
**Status:** 🟡 In Progress  
**Created:** January 23, 2026  
**Supersedes:** 07-BRANDING-HTML-SPRINT.md (archived), 08-VARIABLE-AUDIT.md (merged)

---

## Objective

Fix the configuration propagation pipeline so that branding and settings configured in the TUI or Control Panel actually reach the Gate/HTTP runtime components and are rendered in HTML templates.

### Core Problem
Currently, Gate and HTTP components use `BrandingVars::default()` (8 places) instead of reading user-configured values. This means:
- Custom service names don't appear on pages
- Custom colors don't apply
- All templates show "Fortify" hardcoded defaults

---

## Scope Decisions (January 23, 2026)

### ✅ KEEP (User-Configurable)
| Setting | Default | Notes |
|---------|---------|-------|
| `service_name` | "Protected Service" | Brand name displayed on all pages |
| `description` | "A Fortify-protected onion service" | Tagline |
| `primary_color` | "#c9a227" (Gold) | Main brand color |
| `secondary_color` | "#a68b5b" (Muted gold) | Accent color |
| `welcome_message` | "Please complete the verification to continue." | CAPTCHA page message |
| `pool_size` | 500 | CAPTCHA pool size |
| `min_pool_size` | 100 | Minimum pool before regeneration |
| `max_pool_size` | 1000 | Maximum pool cap |
| `difficulty` | 5 | CAPTCHA difficulty (1-10) |
| `timeout_seconds` | 120 | CAPTCHA solve timeout |
| `max_attempts` | 3 | CAPTCHA attempts before demotion |
| `gate_captcha_type` | (system default) | User can select preferred type |
| `threat_captcha_type` | (system default) | User can select preferred type |
| `random_cycling` | true | Enable random CAPTCHA type cycling |
| `cycle_*` toggles | all true | Which types to include in cycling |
| Behavioral thresholds | (current defaults) | Advanced users can tune |

### ❌ REMOVE (Deprecated)
| Setting | Reason |
|---------|--------|
| `tertiary_color` | Only two colors needed (primary + secondary) |
| `custom_css` | Security risk, unnecessary complexity |
| `audio_enabled` | Not implemented |
| `logo_path` / `logo_base64` | Text-only branding |
| `logo_max_width` / `logo_max_height` | No logo support |

### 🔄 SYSTEM-MANAGED
| Setting | Behavior |
|---------|----------|
| `rotation_percent` | System applies random per-session (20-45% range) |
| `rotation_interval_days` | System manages automatically |

---

## Current Defaults (MUST PRESERVE)

### BrandingConfig (fortify-tui/src/config.rs)
```rust
service_name: "Protected Service"
description: "A Fortify-protected onion service"
primary_color: "#c9a227"   // Gold
secondary_color: "#a68b5b" // Muted gold
welcome_message: "Please complete the verification to continue."
```

### CaptchaConfig
```rust
enabled: true
pool_size: 500
min_pool_size: 100
max_pool_size: 1000
difficulty: 5
timeout_seconds: 120
max_attempts: 3
rotation_percent: 25
rotation_interval_days: 10
```

### ThresholdConfig
```rust
rate_limit_rpm: 60
captcha_fail_limit: 5
temp_ban_minutes: 30
perm_ban_threshold: 10
suspicion_threshold: 0.5
threat_threshold: 0.7
burn_threshold: 0.7
auto_ban_enabled: true
ddos_rps_threshold: 100
probe_sensitivity: 5
```

### NetworkConfig
```rust
backend_address: "http://127.0.0.1:9000"
socks_port: 9150
control_port: 9151
http_bind: "127.0.0.1:8082"
gate_bind: "127.0.0.1:8081"
vanguards_enabled: true
vanguards_layer2: 4
vanguards_layer3: 8
data_dir: ~/.local/share/fortify
```

### MirrorConfig
```rust
min_mirrors: 2
max_mirrors: 5
standby_mirrors: 2
rotation_interval_seconds: 3600
proactive_burn_enabled: true
burn_interval_days_min: 60
burn_interval_days_max: 120
retirement_page_hours: 72
```

### VanityConfig
```rust
enabled: false
prefix: ""
safety_net_enabled: true
safety_net_timeout_seconds: 30
min_prefix_length: 1
warn_threshold: 5
```

---

## Implementation Plan

### Phase 1: Remove Deprecated Fields
**Status:** ⬜ Not Started

1. Remove from `BrandingConfig`:
   - `tertiary_color`
   - `custom_css`
   - `logo_path`, `logo_base64`, `logo_max_width`, `logo_max_height`

2. Remove from `CaptchaConfig`:
   - `audio_enabled`

3. Update `BrandingVars` in fortify-core to match (remove `tertiary_color`)

4. Update HTML templates to not use `{{TERTIARY_COLOR}}`

5. Update Control Panel form to remove deprecated inputs

### Phase 2: Config File Propagation
**Status:** ⬜ Not Started

1. Create shared config file path: `~/.local/share/fortify/config/fortify.toml`

2. TUI saves config to this file on changes

3. Controller passes config path to Gate/HTTP via environment variable

4. Gate reads branding from config file on startup

5. HTTP reads branding from config file on startup

### Phase 3: Replace Hardcoded Defaults
**Status:** ⬜ Not Started

Replace `BrandingVars::default()` with config-loaded values in:

| File | Line | Count |
|------|------|-------|
| `fortify-gate/src/server.rs` | multiple | 6 |
| `fortify-gate/src/captcha_html.rs` | | 1 |
| `fortify-http/src/lib.rs` | | 1 |

### Phase 4: Fix Hardcoded URLs
**Status:** ⬜ Not Started

Replace hardcoded `/Fortify/Portcullis` with configurable path in:

| File | Line | Current |
|------|------|---------|
| `assets/html/busy.html` | 6 | `<title>Fortify — Service Busy</title>` |
| `assets/html/captcha.html` | 193 | `/Fortify` link |
| `assets/html/demoted.html` | 205 | `/Fortify/Portcullis` |
| `assets/html/error.html` | 180 | `/Fortify/Portcullis` |
| `assets/html/gate.html` | 174 | `/Fortify/Portcullis` |
| `assets/html/session-expired.html` | 140 | `/Fortify/Portcullis` |
| `assets/html/verification-failed.html` | 174 | `/Fortify/Portcullis` |

Solution: Add `{{GATE_PATH}}` placeholder, default to `/gate`

### Phase 5: Sync Defaults
**Status:** ⬜ Not Started

Ensure defaults match between:
- `BrandingConfig` (TUI) → `BrandingVars` (Core)
- `CaptchaConfig` (TUI) → `CaptchaConfig` (Gate)

Current mismatch:
- TUI: `service_name: "Protected Service"`
- Core: `service_name: "Fortify"`

Resolution: Use "Protected Service" as default everywhere.

---

## Testing Checklist

### Phase 1 Tests
- [ ] Build succeeds after removing deprecated fields
- [ ] Existing configs load without error (backwards compat)
- [ ] TUI settings page doesn't show removed fields
- [ ] Control Panel doesn't show removed fields

### Phase 2 Tests
- [ ] Config file is created when TUI saves
- [ ] Config file is read by Gate on startup
- [ ] Config file is read by HTTP on startup
- [ ] Changes in TUI appear in Gate after restart

### Phase 3 Tests
- [ ] Custom service_name appears on gate.html
- [ ] Custom primary_color applies to CSS
- [ ] Custom secondary_color applies to CSS
- [ ] Custom welcome_message appears on CAPTCHA page

### Phase 4 Tests
- [ ] No "Fortify" text appears when brand is "TESTBRAND"
- [ ] All links use {{GATE_PATH}} correctly
- [ ] Title tags use {{SERVICE_NAME}}

### Phase 5 Tests
- [ ] Fresh install uses correct defaults
- [ ] Defaults match between TUI and runtime

---

## Files Changed

### Phase 1
- `crates/fortify-tui/src/config.rs` - Remove fields
- `crates/fortify-core/src/branding.rs` - Remove tertiary
- `crates/fortify-core/src/templates.rs` - Remove tertiary from BrandingVars
- `crates/fortify-http/src/admin.rs` - Remove form inputs
- `assets/html/*.html` - Remove TERTIARY_COLOR usage

### Phase 2
- `crates/fortify-tui/src/config.rs` - Add save_to_shared_path()
- `crates/fortify-gate/src/lib.rs` - Add config loading
- `crates/fortify-http/src/lib.rs` - Add config loading
- `crates/fortify-controller/src/main.rs` - Pass config path

### Phase 3
- `crates/fortify-gate/src/server.rs` - Use loaded branding
- `crates/fortify-gate/src/captcha_html.rs` - Use loaded branding
- `crates/fortify-http/src/lib.rs` - Use loaded branding

### Phase 4
- `assets/html/*.html` - Add GATE_PATH placeholder
- `crates/fortify-core/src/templates.rs` - Add GATE_PATH to BrandingVars

---

## Success Criteria

1. ✅ User sets `service_name: "TESTBRAND"` in TUI
2. ✅ User restarts Fortify
3. ✅ All pages show "TESTBRAND" instead of "Fortify"
4. ✅ Custom colors appear on all pages
5. ✅ No "Fortify" branding visible anywhere
6. ✅ All existing defaults preserved for fresh installs

---

## Related Documents

- [07-BRANDING-HTML-SPRINT.md](archive/07-BRANDING-HTML-SPRINT.md) - Archived, merged here
- [08-VARIABLE-AUDIT.md](08-VARIABLE-AUDIT.md) - Merged, will archive after completion
- [DEV-PROGRESS-AUDIT.md](DEV-PROGRESS-AUDIT.md) - Sprint tracking
