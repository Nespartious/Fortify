# Sprint 12: Complete Template Migration

**Status**: ✅ COMPLETE  
**Branch**: `feature/combined-templates-branding`  
**Started**: 2025-01-22  
**Completed**: 2025-01-22  
**Depends On**: Sprint 11 (Static CAPTCHA & Templates Engine)

## Overview

This sprint completes the migration of ALL inline HTML to the template engine system established in Sprint 11. This ensures consistent citadel/gold branding across all user-facing pages and eliminates legacy neon/synthwave styling.

## Design Principles

1. **No Inline HTML** - All HTML pages must use template engine
2. **Consistent Branding** - Citadel/gold theme (`--brand-primary: #c9a227`) everywhere
3. **Zero Legacy CSS** - Remove all neon-pink/neon-cyan/synthwave references
4. **Template-First** - Prefer templates over format!() HTML strings

---

## Phase 1: New Template Types ✅

**Status**: COMPLETE

### Objective
Create new HTML templates and add corresponding TemplateType variants.

### Templates Created

| Template | Purpose | Variables |
|----------|---------|-----------|
| `verification-failed.html` | CAPTCHA/verification failure with retry | `ATTEMPTS`, `DELAY_SECONDS`, `DELAY_DISPLAY` |
| `session-expired.html` | Session killed/burned notification | Standard branding only |

### Files Modified

| File | Change |
|------|--------|
| `assets/html/verification-failed.html` | ✅ Created with citadel/gold theme |
| `assets/html/session-expired.html` | ✅ Created with citadel/gold theme |
| `fortify-core/src/templates.rs` | ✅ Added `VerificationFailed` and `SessionExpired` variants |

---

## Phase 2: Gate Server Migration ✅

**Status**: COMPLETE

### Objective
Replace all inline HTML in `fortify-gate/src/server.rs` with template engine calls.

### Functions Migrated

| Function | Target Template | Status |
|----------|-----------------|--------|
| `serve_demoted_page()` fallback | Captcha | ✅ Uses template engine |
| Fallback CAPTCHA page | Captcha | ✅ Uses template engine |
| Verification failed page | VerificationFailed | ✅ Uses template engine |
| `styled_error_response()` | Error | ✅ Uses template engine |

---

## Phase 3: CAPTCHA HTML Migration ✅

**Status**: COMPLETE

### Objective
Update `captcha_css()` and remaining CAPTCHA type renderers to use new theme.

### Changes Made

| File | Function | Change |
|------|----------|--------|
| `fortify-gate/src/captcha_html.rs` | `captcha_css()` | ✅ Updated to citadel/gold theme |

---

## Phase 4: HTTP Crate Migration ✅

**Status**: COMPLETE

### Objective
Migrate `fortify-http` inline HTML to template engine.

### Functions Migrated

| Function | Target Template | Status |
|----------|-----------------|--------|
| `serve_killed_session_page()` | SessionExpired | ✅ Uses template engine |
| `serve_paused_redirect_page()` | Inline (citadel theme) | ✅ Updated to citadel/gold |

---

## Phase 4b: Node & Orchestrator Migration ✅

**Status**: COMPLETE

### Objective
Update remaining inline HTML in `fortify-node` and `fortify-orchestrator` to citadel/gold theme.

### Functions Migrated

| File | Function | Status |
|------|----------|--------|
| `fortify-node/src/lib.rs` | `serve_backend_fallback()` | ✅ Updated to citadel/gold |
| `fortify-node/src/lib.rs` | `redirect_to_gate()` | ✅ Updated to citadel/gold |
| `fortify-orchestrator/src/server.rs` | `serve_maintenance_page()` | ✅ Updated to citadel/gold |

---

## Phase 4c: Timer CSS Migration ✅

**Status**: COMPLETE

### Objective
Update `timer_css()` in CAPTCHA HTML to use citadel/gold theme variables.

### Changes Made

| File | Function | Status |
|------|----------|--------|
| `fortify-gate/src/captcha_html.rs` | `timer_css()` | ✅ All neon refs replaced with citadel/gold |

---

## Phase 5: Verification & Documentation ✅

**Status**: COMPLETE

### Checklist

- [x] All inline HTML using citadel/gold theme
- [x] Zero `neon-pink`, `neon-cyan`, `#0d0211` references in `.rs` files
- [x] Project builds with zero errors
- [x] Project builds with zero warnings
- [x] Sprint 11 document updated with completion status
- [x] This document archived to `archive/`
- [x] All templates use consistent citadel/gold theme

---

## Progress Log

### 2025-01-22

- Created Sprint 12 dev progress document
- Identified 5 inline HTML locations requiring migration
- Starting Phase 1: New Template Types
- **Phase 1 Complete**: Created `VerificationFailed` and `SessionExpired` templates
- **Phase 2 Complete**: Migrated gate server inline HTML (4 functions)
- **Phase 3 Complete**: Updated `captcha_css()` to citadel/gold
- **Phase 4 Complete**: Migrated `serve_killed_session_page()` and `serve_paused_redirect_page()`
- **Phase 4b Complete**: Updated fortify-node inline HTML (`serve_backend_fallback()`, `redirect_to_gate()`)
- **Phase 4c Complete**: Updated orchestrator `serve_maintenance_page()` 
- **Phase 5 Complete**: All verification checks passed, build clean
- **SPRINT COMPLETE**: Zero legacy theme references remain in codebase

### 2025-01-23

- **Bug Fix**: Demoted user captcha loop - users solving 2 captchas were sent back to solve 2 more
- **Root Cause**: Verification token upgrade block condition `if verified_session_id.is_none()` caused demoted users (who have valid session tokens) to skip the upgrade entirely
- **Fix Applied**: Changed condition to `if verification_token_opt.is_some()` - verification tokens now ALWAYS take priority
- **Additional Fix**: Clear tier overrides for ALL related sessions (new, original, demoted, stale)
- **Verification**: New user flow (1 captcha) and demoted user flow (2 captchas) both working correctly
- **Files Modified**: `fortify-http/src/lib.rs` - verification token handling logic

---

## Files Modified Summary

| File | Changes |
|------|---------|
| `assets/html/verification-failed.html` | Created |
| `assets/html/session-expired.html` | Created |
| `assets/html/error.html` | Added dynamic placeholders |
| `fortify-core/src/templates.rs` | Added 2 new template types |
| `fortify-gate/src/server.rs` | 4 functions migrated to templates |
| `fortify-gate/src/captcha_html.rs` | `captcha_css()` and `timer_css()` updated |
| `fortify-http/src/lib.rs` | 2 functions migrated/updated |
| `fortify-node/src/lib.rs` | 2 functions updated to citadel theme |
| `fortify-orchestrator/src/server.rs` | 1 function updated to citadel theme |

---

## Color Reference

### Legacy (Remove)
```css
--neon-pink: #ff2a6d or #d500f9
--neon-cyan: #05d9e8 or #00e5ff
--bg-color: #0d0211
```

### New (Citadel/Gold)
```css
--bg-deep: #141417;
--bg-surface: #1e1e23;
--bg-elevated: #26262d;
--border-subtle: #3a3a42;
--brand-primary: #c9a227;
--gold-muted: #a68b5b;
--text-primary: #f5f0e8;
--text-secondary: #a8a4a0;
--text-muted: #6b6862;
--accent-red: #e05252;
--accent-amber: #e4bc5e;
```
