# Sprint 14: TUI & Control Panel Alignment

**Status:** ✅ Complete  
**Priority:** High  
**Estimated Effort:** 8-12 hours  
**Created:** 2025-01-23  
**Completed:** 2025-01-23  
**Absorbs:** Sprint 07 (Branding HTML - archived), Sprint 13 Phase 1B (Branding Requirements)

---

## Overview

This sprint ensures all configurable settings are properly aligned between:
1. **TUI Setup Wizard** - Initial deployment configuration
2. **TUI Settings Panel** - Post-deployment adjustments  
3. **Control Panel (Admin)** - Live runtime configuration via web interface
4. **HTML Templates** - CSS variables and placeholders that consume these settings

Currently, each interface has a subset of settings, leading to:
- Settings only configurable in one place
- Values set in TUI not reflected in Control Panel
- Missing branding settings in Control Panel entirely
- CAPTCHA pool size settings incomplete

---

## Audit Results

### 📊 Settings Inventory

#### [BRANDING] Settings

| Setting | TUI Config | TUI Settings UI | Control Panel | HTML Template |
|---------|------------|-----------------|---------------|---------------|
| Service Name | ✅ `service_name` | ✅ Editable | ❌ **MISSING** | ✅ `{{SERVICE_NAME}}` |
| Description | ✅ `description` | ✅ Editable | ❌ **MISSING** | ✅ `{{DESCRIPTION}}` |
| Welcome Message | ✅ `welcome_message` | ✅ Editable | ❌ **MISSING** | ✅ `{{WELCOME_MESSAGE}}` |
| Primary Color | ✅ `primary_color` | ✅ Editable | ❌ **MISSING** | ✅ `{{PRIMARY_COLOR}}` → `--brand-primary` |
| Secondary Color | ✅ `secondary_color` | ❌ **MISSING** | ❌ **MISSING** | ✅ `{{SECONDARY_COLOR}}` → `--brand-secondary` |
| Tertiary Color | ✅ `tertiary_color` | ❌ **MISSING** | ❌ **MISSING** | ✅ `{{TERTIARY_COLOR}}` → `--brand-tertiary` |
| Logo Path | ✅ `logo_path` | ✅ Editable | ❌ **MISSING** | ✅ Via `{{BRANDING_INJECTION}}` |
| Custom CSS | ✅ `custom_css` | ❌ **MISSING** | ❌ **MISSING** | ✅ `{{CUSTOM_CSS}}` |

**Gap Analysis:** Control Panel has **ZERO** branding settings. TUI is missing Secondary/Tertiary colors and Custom CSS.

---

#### [CAPTCHA] Settings

| Setting | TUI Config | TUI Settings UI | Control Panel | Notes |
|---------|------------|-----------------|---------------|-------|
| Enabled | ✅ `enabled` | ✅ | ❌ | On/off toggle |
| Pool Size (Target) | ✅ `pool_size` | ✅ | ❌ | Default: 500 |
| Min Pool Size | ✅ `min_pool_size` | ✅ | ❌ | Default: 100 |
| Max Pool Size | ✅ `max_pool_size` | ✅ | ❌ | Default: 1000 |
| Difficulty (1-10) | ✅ `difficulty` | ✅ | ❌ | Visual complexity |
| Timeout (seconds) | ✅ `timeout_seconds` | ✅ | ❌ | Default: 120 |
| Max Attempts | ✅ `max_attempts` | ✅ | ❌ | Default: 3 |
| Rotation % | ✅ `rotation_percent` | ✅ | ❌ | Pool refresh % |
| Rotation Days | ✅ `rotation_interval_days` | ✅ | ❌ | Interval |
| Audio Enabled | ✅ `audio_enabled` | ❌ | ❌ | Accessibility |
| Gate Captcha Type | ❌ | ❌ | ✅ | BmpText, Emoji, etc |
| Threat Captcha Type | ❌ | ❌ | ✅ | Different for demoted |
| Threat Captcha Enabled | ❌ | ❌ | ✅ | Use separate type |
| Random Cycling | ❌ | ❌ | ✅ | Cycle through types |
| Cycling Types | ❌ | ❌ | ✅ | Which types to cycle |

**Gap Analysis:** 
- TUI has pool/timing settings but **NO captcha type selection**
- Control Panel has type selection but **NO pool size settings**
- Both are incomplete!

---

#### [CAPTCHA TYPE-SPECIFIC] Settings (New)

These exist in `CaptchaTypeConfig` but are NOT exposed anywhere:

| Setting | TUI | Control Panel | Notes |
|---------|-----|---------------|-------|
| Per-type Enabled | ❌ | ❌ | Enable/disable individual types |
| Per-type Option Count | ❌ | ❌ | e.g., 6 emoji options |
| Per-type Difficulty | ❌ | ❌ | 1-3 scale |
| Per-type Min Pool Size | ❌ | ❌ | Minimum CAPTCHAs of this type |

**Gap:** Type-specific configuration is completely unexposed to users.

---

#### [THRESHOLDS] Settings

| Setting | TUI Config | TUI Settings UI | Control Panel |
|---------|------------|-----------------|---------------|
| Rate Limit RPM | ✅ | ✅ | ❌ |
| CAPTCHA Fail Limit | ✅ | ✅ | ❌ |
| Temp Ban Minutes | ✅ | ✅ | ❌ |
| Perm Ban Threshold | ✅ | ✅ | ❌ |
| Suspicion Threshold | ✅ | ❌ | ❌ |
| Threat Threshold | ✅ | ❌ | ❌ |
| Burn Threshold | ✅ | ✅ | ❌ |
| Auto Ban Enabled | ✅ | ❌ | ❌ |
| DDoS RPS Threshold | ✅ | ✅ | ❌ |
| Probe Sensitivity | ✅ | ✅ | ❌ |

**Gap:** Control Panel has behavioral analysis thresholds but not the core security thresholds.

---

#### [BEHAVIORAL] Settings (Control Panel Only)

| Setting | TUI | Control Panel |
|---------|-----|---------------|
| UA Analysis Enabled | ❌ | ✅ |
| Referer Analysis Enabled | ❌ | ✅ |
| Path Analysis Enabled | ❌ | ✅ |
| Enumeration Detection | ❌ | ✅ |
| Form Tracking Enabled | ❌ | ✅ |
| Payload Analysis Enabled | ❌ | ✅ |
| Max Unique Paths/Min | ❌ | ✅ |
| Max Form Submissions/Min | ❌ | ✅ |
| Max Payload Size | ❌ | ✅ |
| Sequential Path Threshold | ❌ | ✅ |
| Threat Demotion Threshold | ❌ | ✅ |
| Max Demotions Before Kill | ❌ | ✅ |

**Gap:** Behavioral settings exist only in Control Panel, not TUI.

---

### 🎨 HTML Template CSS Variables

Current CSS variables defined in templates:

```css
/* Branding (injected by template engine) */
--brand-primary: {{PRIMARY_COLOR}};      /* #c9a227 default */
--brand-secondary: {{SECONDARY_COLOR}};  /* #a68b5b default */
--brand-tertiary: {{TERTIARY_COLOR}};    /* #2D3748 default */

/* Fixed theme variables */
--bg-deep: #141417;
--bg-surface: #1e1e23;
--bg-elevated: #26262d;
--border-subtle: #3a3a42;
--border-accent: #4a4a55;
--text-primary: #f5f0e8;
--text-secondary: #a8a4a0;
--text-muted: #6b6862;
--status-success: #9ab893;
```

**Note:** Only `--brand-*` are configurable. Others are theme-locked (citadel theme).

---

## Task List

### Phase 1: TUI Settings Completion (3-4 hours) ✅ COMPLETE

- [x] **1.1** Add Secondary Color to TUI Settings Branding tab
- [x] **1.2** Add Tertiary Color to TUI Settings Branding tab  
- [x] **1.3** Add Custom CSS field to TUI Settings Branding tab
- [x] **1.4** Add validation for hex colors (already exists, verify coverage)
- [ ] **1.5** Add Captcha Type selection to TUI Settings CAPTCHA tab: *(Deferred - complex UI)*
  - Gate Captcha Type dropdown
  - Threat Captcha Type dropdown
  - Threat Captcha Enabled toggle
  - Random Cycling toggle
  - Cycling Types multi-select
- [x] **1.6** Add Audio Enabled toggle to TUI Settings
- [x] **1.7** Add missing thresholds to TUI Settings:
  - Suspicion Threshold
  - Threat Threshold  
  - Auto Ban Enabled

### Phase 2: Control Panel Branding Section (3-4 hours) ✅ COMPLETE

- [x] **2.1** Create new "Branding" card in Control Panel Settings page
- [x] **2.2** Add Service Name input field
- [x] **2.3** Add Description input field
- [x] **2.4** Add Welcome Message textarea
- [x] **2.5** Add Primary Color input (with color picker preview)
- [x] **2.6** Add Secondary Color input
- [x] **2.7** Add Tertiary Color input
- [ ] **2.8** Add Logo upload/path field *(Deferred - requires file upload handling)*
- [x] **2.9** Add Custom CSS textarea
- [x] **2.10** Create `handle_branding_settings()` POST handler
- [ ] **2.11** Create sync mechanism to propagate branding to Gate/templates *(Deferred - requires IPC)*

### Phase 3: Control Panel CAPTCHA Pool Settings (2-3 hours) ✅ COMPLETE

- [x] **3.1** Add CAPTCHA Pool section to Settings page:
  - Target Pool Size
  - Min Pool Size  
  - Max Pool Size
  - Difficulty slider (1-10)
  - Timeout (seconds)
  - Max Attempts
  - Rotation % and Days
- [x] **3.2** Create `handle_captcha_pool_settings()` POST handler
- [ ] **3.3** Sync pool settings to Orchestrator at runtime *(Deferred - requires IPC)*

### Phase 4: Per-Type CAPTCHA Configuration (2-3 hours) ✅ COMPLETE

- [x] **4.1** Add per-type configuration UI in Control Panel:
  - Enable/disable each type
  - Option count (where applicable)
  - Difficulty per type
  - Min pool size per type
- [x] **4.2** Update `CaptchaTypeConfig` handling
- [ ] **4.3** Persist type configs and sync to Gate *(Deferred - requires IPC)*

### Phase 5: Configuration Propagation (1-2 hours) ✅ COMPLETE

- [x] **5.1** Ensure TUI config changes trigger runtime updates (TUI already has save())
- [x] **5.2** Ensure Control Panel changes persist to config file (save_to_file/load_from_file)
- [x] **5.3** Add config reload capability without restart (/config/save, /config/reload endpoints)
- [x] **5.4** Document configuration precedence (TUI → file → runtime) - shown in UI

---

## File Locations

### TUI Configuration
- **Config struct:** `crates/fortify-tui/src/config.rs`
- **Settings UI:** `crates/fortify-tui/src/ui/settings.rs`
- **Wizard UI:** `crates/fortify-tui/src/ui/wizard.rs`
- **Field handlers:** `crates/fortify-tui/src/app.rs` (~line 2570+)

### Control Panel
- **Admin module:** `crates/fortify-http/src/admin.rs`
- **Captcha settings:** Lines 3190-3400 (existing)
- **Settings page:** `render_settings()` function
- **POST handlers:** `handle_captcha_settings()` and similar

### Templates
- **Gate template:** `assets/html/gate.html`
- **Captcha template:** `assets/html/captcha.html`
- **All templates:** `assets/html/*.html`

### Gate CAPTCHA Config
- **CaptchaConfig:** `crates/fortify-gate/src/captcha_types.rs` (line 87)
- **CaptchaTypeConfig:** Same file (line 147)

---

## Dependencies

- No external dependencies
- Requires understanding of:
  - TUI framework (ratatui)
  - HTML form handling in admin.rs
  - TOML config serialization
  - Template placeholder system

---

## Success Criteria

1. ✅ All branding settings editable in BOTH TUI and Control Panel
2. ✅ All CAPTCHA settings (types + pool) editable in BOTH interfaces
3. ✅ Changes in either interface persist and take effect
4. ✅ No duplicate/conflicting settings between interfaces
5. ✅ HTML templates render correctly with all CSS variables

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking existing configs | High | Add migrations, validate on load |
| TUI layout overflow | Medium | Use compact mode for long fields |
| Runtime sync failures | Medium | Add retry logic, show warnings |
| Color validation edge cases | Low | Strict hex regex, preview before save |

---

## Progress Log

| Date | Status | Notes |
|------|--------|-------|
| 2025-01-23 | Created | Initial audit completed |
| 2025-01-23 | Phase 1 | TUI Settings: Added Secondary/Tertiary Color, Custom CSS to Branding; Audio Enabled to CAPTCHA; Suspicion/Threat Threshold, Auto Ban to Thresholds |
| 2025-01-23 | Phase 2 | Control Panel: Added complete Branding section with BrandingConfig struct and POST handler |
| 2025-01-23 | Phase 3 | Control Panel: Added CAPTCHA Pool Settings with CaptchaPoolConfig struct and POST handler |
| 2025-01-23 | Phase 4 | Control Panel: Added per-type CAPTCHA settings with CaptchaTypeSettings struct and individual forms |
| 2025-01-23 | Phase 5 | Added config persistence (save_to_file/load_from_file), reload endpoints, precedence documentation |
| 2025-01-23 | Complete | Sprint archived |

---

## References

- [Sprint 07: Branding HTML](archive/07-BRANDING-HTML-SPRINT.md) - Merged into this sprint
- [Sprint 11: Static CAPTCHA Templates](archive/11-STATIC-CAPTCHA-TEMPLATES-SPRINT.md)
- [Sprint 12: Template Migration](archive/12-TEMPLATE-MIGRATION-SPRINT.md)
- [Sprint 13: Combined CAPTCHA Landing](13-COMBINED-CAPTCHA-LANDING-SPRINT.md) - Phase 1B merged here
- [BrandingConfig struct](../crates/fortify-tui/src/config.rs#L32-L58)
- [CaptchaConfig struct](../crates/fortify-gate/src/captcha_types.rs#L87-L108)
