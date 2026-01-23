# Sprint 17: Settings Hot Reload & Missing Config Fields

**Sprint ID:** BETA-017  
**Priority:** 🔴 HIGH (Core UX)  
**Estimated Effort:** 4-5 days  
**Status:** � In Progress  
**Created:** January 23, 2026  
**Supersedes:** Portions of Sprint 14 (archived), Sprint 15 Phase 6

---

## Objective

1. **Fix Missing Settings** - Add CAPTCHA type configuration and other missing fields to TUI and Config
2. **Implement Hot Reload** - Settings that can be applied live should take effect immediately
3. **Restart Flow** - Settings requiring restart should prompt user with options
4. **UX Improvement** - After saving, return to deployment status screen with proper logging
5. **TUI Navigation Strictness** - Make TUI flow more explicit with clear View/Modify modes

---

## TUI UX Philosophy (User Feedback)

### Problem Statement
The TUI isn't strict enough with its navigation flow. When deployed:
- Accessing "Settings" is confusing - how do I return to the deployed screen?
- No clear separation between viewing settings and modifying them
- "Status" button purpose unclear when already on Running view

### Proposed Solution

**When service is RUNNING (Live TUI Monitor Screen):**

| Action | Current | Proposed |
|--------|---------|----------|
| View settings | Settings → confusing return | **View System Settings** (read-only, "Done" returns to monitor) |
| Modify settings | Settings → edit → ??? | **Modify System Settings** (editable, "Cancel"/"Submit") |
| View status | Status (duplicate?) | Remove or merge into Live TUI Monitor |

**Terminology:**
- "Live TUI Monitor Screen" = `View::Running` (logs right, status left)
- Settings access should be explicit: View vs Modify

**Decision Points (Awaiting User Input):**
1. Should both View/Modify appear in menu when deployed, or is Modify accessed from View mode?
2. Should Status menu item be removed entirely or renamed?
3. Should this strict flow only apply when deployed, or always?

---

## Problem Analysis

### Issue 1: Tier Tab Not Visible in TUI
**Root Cause:** Settings menu defaults to `SettingsTab::Branding` (line 1683 of app.rs)
**Solution:** Change default to `SettingsTab::TrafficTier` or make Tier tab more discoverable

```rust
// Current (line 1683):
self.view = View::Settings {
    tab: SettingsTab::Branding,  // <-- Should be TrafficTier
    field_index: 0,
};
```

### Issue 2: Missing CAPTCHA Type Settings
**Root Cause:** `CaptchaConfig` in fortify-gate has type settings, but `CaptchaConfig` in fortify-tui does not

| Setting | fortify-gate | fortify-tui | TUI UI | Control Panel |
|---------|--------------|-------------|--------|---------------|
| `gate_captcha_type` | ✅ | ❌ | ❌ | ✅ |
| `threat_captcha_type` | ✅ | ❌ | ❌ | ✅ |
| `threat_captcha_enabled` | ✅ | ❌ | ❌ | ✅ |
| `random_cycling` | ✅ | ❌ | ❌ | ✅ |
| `cycling_types` | ✅ | ❌ | ❌ | ✅ |

**Solution:** Add these fields to TUI's `CaptchaConfig` and expose in Settings UI

### Issue 3: Config Changes Don't Apply Live
**Root Cause:** Config is saved to disk but running services don't reload it
**Solution:** Implement hot reload channel from TUI → running services

---

## Settings Classification

### ✅ HOT RELOAD (Apply Immediately)

These settings can be changed without service restart:

| Category | Setting | Notes |
|----------|---------|-------|
| **Branding** | Service Name | Template variable |
| **Branding** | Description | Template variable |
| **Branding** | Welcome Message | Template variable |
| **Branding** | Primary Color | CSS variable |
| **Branding** | Secondary Color | CSS variable |
| **Rate Limits** | Rate Limit RPM | GlobalRateLimiter reads from AdminState |
| **Rate Limits** | Traffic Tier | Updates rate_limit_multiplier() |
| **Thresholds** | Suspicion Threshold | Behavioral scoring |
| **Thresholds** | Threat Threshold | Behavioral scoring |
| **Thresholds** | Temp Ban Duration | Ban timing |
| **CAPTCHA** | Difficulty | Generation parameter |
| **CAPTCHA** | Timeout Seconds | Validation parameter |
| **CAPTCHA** | Max Attempts | Validation parameter |
| **CAPTCHA** | Gate CAPTCHA Type | Selection logic |
| **CAPTCHA** | Threat CAPTCHA Type | Selection logic |
| **CAPTCHA** | Random Cycling | Selection logic |
| **CAPTCHA** | Cycling Types | Selection logic |

### ⚠️ RESTART REQUIRED (Queue for Restart)

These settings require service restart to take effect:

| Category | Setting | Reason |
|----------|---------|--------|
| **CAPTCHA** | Pool Size | Pre-generation at startup |
| **CAPTCHA** | Min Pool Size | Pool management boundaries |
| **CAPTCHA** | Max Pool Size | Pool management boundaries |
| **Mirrors** | Min Mirrors | Tor circuit changes |
| **Mirrors** | Max Mirrors | Tor circuit changes |
| **Mirrors** | Standby Mirrors | Tor circuit changes |
| **Network** | Backend Address | Connection reconfiguration |
| **Network** | SOCKS Port | Tor reconfiguration |
| **Network** | Control Port | Tor reconfiguration |
| **Network** | Vanguards settings | Tor reconfiguration |
| **Vanity** | All settings | Onion address generation |

---

## Implementation Plan

### Phase 1: Fix Tier Tab Visibility (1 hour)
**Status:** ✅ Complete

1. ✅ Changed Settings default tab from Branding to TrafficTier:
   - Line 1683: `MenuItem::Settings` handler
   - Line 2274: `'s'` shortcut from Running view

### Phase 2: Add Missing CAPTCHA Config Fields (4 hours)
**Status:** ✅ Complete

1. ✅ Added `CaptchaType` enum to TUI config.rs with:
   - All 7 types: BmpText, Emoji, Direction, Sequence, WordUnscramble, ImageRotation, Silhouette
   - `display_name()` and `next()` methods for UI cycling

2. ✅ Added new fields to `CaptchaConfig`:
   - `gate_captcha_type: CaptchaType`
   - `threat_captcha_type: CaptchaType`
   - `threat_captcha_enabled: bool`
   - `random_cycling: bool`
   - `cycling_types: Vec<CaptchaType>`

3. ✅ Updated TUI CAPTCHA settings tab to show all new fields with ↵ for toggle/cycle
4. ✅ Added field editing handlers in `apply_field_change()`
5. ✅ Added ⚠️Restart labels to Pool Size, Min Pool, Max Pool fields

### Phase 3: Hot Reload Infrastructure (8 hours)
**Status:** ✅ Complete

1. ✅ Enhanced `DeploymentManager::reload_config()` to push settings via HTTP:
   - Makes POST requests to running AdminState HTTP API
   - Updates branding config via `/settings/branding`
   - Updates traffic tier via `/settings/traffic-tier`
   - Graceful fallback if HTTP fails (logs warning, continues)
   
2. ✅ Added `urlencoding` dependency for form data encoding

3. Settings pushed on hot reload:
   - Branding: service_name, description, primary_color, secondary_color, welcome_message
   - Traffic tier: micro/small/medium/large/enterprise

### Phase 4: Restart Prompt Dialog (4 hours)
**Status:** ✅ Complete

1. ✅ Modified `Dialog::ApplyChanges` enum to separate hot-reload vs restart-required:
   ```rust
   Dialog::ApplyChanges {
       hot_reload: Vec<String>,
       restart_required: Vec<String>,
   }
   ```

2. ✅ Added `PendingChange::requires_restart()` method classifying 18+ settings:
   - Pool sizes, mirrors, network settings, vanity, vanguards

3. ✅ Added `ChangeManager` methods:
   - `hot_reload_changes()` - filter to hot-reload only
   - `restart_required_changes()` - filter to restart-required only
   - `has_restart_required()` - check if any need restart
   - `clear()` - discard all changes

4. ✅ Updated dialog UI to show categorized changes:
   - ✓ Green section for "Can apply immediately"
   - ⚠ Yellow section for "Requires restart"

5. ✅ Dialog options based on change types:
   - `[A]` Apply hot-reload changes only
   - `[R]` Restart to apply all changes
   - `[C]` Cancel (discard all changes)

6. ✅ Post-action navigation:
   - Apply: Returns to `View::Running` (Live TUI Monitor)
   - Restart: Stops services, returns to `View::Home` for redeploy
   - Cancel: Stays in Settings

### Phase 5: Post-Save Navigation (2 hours)
**Status:** ✅ Complete (merged into Phase 4)

1. ✅ After successful apply:
   - Return to `View::Running` (deployment status)
   - Show toast: "Changes applied successfully" or "Applied X changes. Y require restart."
   - Left panel: Status summary
   - Right panel: Live logs

2. Add toast notification system:
   ```rust
   pub struct Toast {
       message: String,
       level: ToastLevel,
       expires_at: Instant,
   }
   ```

---

## Files to Modify

### Phase 1
- `crates/fortify-tui/src/app.rs` - Default tab change

### Phase 2
- `crates/fortify-tui/src/config.rs` - Add CaptchaType enum, update CaptchaConfig
- `crates/fortify-tui/src/ui/settings.rs` - Add CAPTCHA type UI
- `crates/fortify-tui/src/app.rs` - Handle CAPTCHA type field editing

### Phase 3
- `crates/fortify-core/src/lib.rs` - Add ConfigUpdate channel types
- `crates/fortify-gate/src/lib.rs` - Add channel receiver
- `crates/fortify-http/src/lib.rs` - Add channel receiver
- `crates/fortify-controller/src/lib.rs` - Create and distribute channels
- `crates/fortify-tui/src/app.rs` - Send updates on save

### Phase 4
- `crates/fortify-tui/src/app.rs` - Restart dialog logic
- `crates/fortify-tui/src/ui/dialog.rs` - Restart dialog rendering
- `crates/fortify-tui/src/config.rs` - Pending changes tracking

### Phase 5
- `crates/fortify-tui/src/app.rs` - Post-save navigation
- `crates/fortify-tui/src/ui/mod.rs` - Toast rendering
- `crates/fortify-tui/src/app.rs` - Toast state management

---

## Testing Checklist

### Phase 1
- [ ] Settings menu now opens to Tier tab
- [ ] All tabs still navigable with ←/→

### Phase 2
- [x] CAPTCHA tab shows Gate CAPTCHA Type field
- [x] CAPTCHA tab shows Threat CAPTCHA Enable toggle
- [x] CAPTCHA tab shows Threat CAPTCHA Type (when enabled)
- [x] CAPTCHA tab shows Random Cycling toggle
- [x] CAPTCHA tab shows Cycling Types (display only for now)

### Phase 3
- [ ] Branding changes apply immediately without restart
- [ ] Color changes visible on next page load
- [ ] Traffic tier changes take effect immediately

### Phase 4
- [ ] Changing pool size triggers restart dialog
- [ ] "Apply & Restart Now" restarts services
- [ ] "Stage for Next Restart" saves without restart
- [ ] "Cancel" discards restart-required changes only

### Phase 5
- [ ] After save, returns to Running view
- [ ] Toast shows confirmation message
- [ ] Status panel on left, logs on right

### Phase 6 (TUI UX Strictness) - AWAITING DECISIONS
- [ ] View System Settings (read-only mode)
- [ ] Modify System Settings (edit mode)
- [ ] Clear return path to Live TUI Monitor
- [x] Status button removed or clarified

---

## Success Criteria

1. ✅ User can access Tier tab from Settings (it's the default)
2. ✅ User can configure CAPTCHA types in TUI
3. ✅ Branding/rate limit changes apply without restart (via HTTP hot-reload)
4. ✅ Pool size changes prompt for restart confirmation (categorized dialog)
5. ✅ After saving, user sees deployment status with logs (returns to Running view)
6. ✅ TUI navigation is strict with View/Modify separation

---

## Related Documents

- [15-BRANDING-CONFIG-PROPAGATION-SPRINT.md](15-BRANDING-CONFIG-PROPAGATION-SPRINT.md) - Previous config work
- [16-TIER-INTEGRATION-SPRINT.md](16-TIER-INTEGRATION-SPRINT.md) - Tier implementation
- [archive/14-TUI-CP_ALIGNMENT-SPRINT.md](archive/14-TUI-CP_ALIGNMENT-SPRINT.md) - Settings audit

---

## Phase 6: TUI UX Strictness (Proposed)
**Status:** � In Progress

### User Decisions (Confirmed)

1. **Option A: Two separate menu items**
   - "View System Settings" (read-only)
   - "Modify System Settings" (editable)

2. **Merge Status into Live TUI Monitor** - Remove `MenuItem::Status`

3. **Apply strictness always** - Not just when deployed

### Implementation Plan

**Menu Changes:**
- Remove: `MenuItem::Status` 
- Replace: `MenuItem::Settings` → `MenuItem::ViewSettings` + `MenuItem::ModifySettings`

**View System Settings (read-only):**
- All settings visible but greyed/non-editable
- Navigation: Tab left/right, Up/down to scroll
- Exit: **[Esc]** or **[Enter]** → returns to previous view

**Modify System Settings (edit mode):**
- All settings editable (current behavior)
- Exit without changes: **[Esc]** → "Discard changes?" confirmation
- Save changes: **[Enter]** on Submit → Apply dialog → returns to Live TUI Monitor (if running) or Home

**Navigation Flow:**
```
Home (pre-deploy)           Running (post-deploy)
  ├── Deploy                  ├── Live TUI Monitor (status + logs)
  ├── View Settings           ├── View Settings → [Esc] → Live TUI Monitor
  ├── Modify Settings         ├── Modify Settings → Submit → Live TUI Monitor  
  └── Quit                    └── Undeploy
```

### Files to Modify

- `crates/fortify-tui/src/app.rs`:
  - Add `MenuItem::ViewSettings`, `MenuItem::ModifySettings`
  - Remove `MenuItem::Status`
  - Add `View::ViewSettings` variant (read-only mode)
  - Update menu rendering and handlers

- `crates/fortify-tui/src/ui/menu.rs`:
  - Update menu item labels

- `crates/fortify-tui/src/ui/settings.rs`:
  - ✅ Added read-only mode detection via View::ViewSettings
  - ✅ Different footer for read-only (Done) vs edit (Edit/Back)
  - ✅ Different title for read-only vs edit mode

---

## Appendix: Full Settings Gap Analysis

### Missing in TUI Config (exist in Gate but not TUI)

| Setting | Location in Gate | Priority | Status |
|---------|------------------|----------|--------|
| `gate_captcha_type` | `captcha_types.rs:89` | HIGH | ✅ Added |
| `threat_captcha_type` | `captcha_types.rs:91` | HIGH | ✅ Added |
| `threat_captcha_enabled` | `captcha_types.rs:93` | HIGH | ✅ Added |
| `random_cycling` | `captcha_types.rs:95` | MEDIUM | ✅ Added |
| `cycling_types` | `captcha_types.rs:97` | MEDIUM | ✅ Added |
| `type_configs` (per-type settings) | `captcha_types.rs:99` | LOW | ⬜ Future |

### Settings with ⚠️ Restart labels (already marked)

| Setting | Tab | Current Label |
|---------|-----|---------------|
| CAPTCHA Pool | Tier | "⚠️Restart" |
| Pool Size | CAPTCHA | ✅ "⚠️Restart" |
| Min Pool | CAPTCHA | ✅ "⚠️Restart" |
| Max Pool | CAPTCHA | ✅ "⚠️Restart" |

### Settings that need validation

| Setting | Validation |
|---------|------------|
| Primary Color | Hex format #RRGGBB |
| Secondary Color | Hex format #RRGGBB |
| Rate Limit RPM | > 0 |
| Difficulty | 1-10 |
| Timeout Seconds | > 0 |
| Pool sizes | min < pool < max |
