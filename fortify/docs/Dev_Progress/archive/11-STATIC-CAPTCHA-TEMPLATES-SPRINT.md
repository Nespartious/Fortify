# Sprint 11: Static CAPTCHA & Templates Engine

**Status**: ✅ COMPLETE  
**Branch**: `feature/static-templates-engine`  
**Started**: 2025-01-23  
**Completed**: 2025-01-22 (Sprint 12 completed full migration)  

## Continuation

**See Also**: [Sprint 12 - Template Migration](12-TEMPLATE-MIGRATION-SPRINT.md) completes the migration of ALL inline HTML to this template engine.

## Overview

This sprint implements a compile-time template engine and pre-rendered CAPTCHA page system for maximum DDoS resilience. The goal is to eliminate runtime HTML generation overhead and enable edge-caching of static pages.

## Design Principles

1. **No JavaScript** - Tor users distrust JS; all pages are pure HTML/CSS
2. **Compile-time Embedding** - Templates loaded via `include_str!()` for zero disk I/O
3. **Pre-rendered CAPTCHAs** - Complete HTML pages generated during quiet periods
4. **Edge-cache Ready** - Static pages can be served from nginx at 100K+ req/sec

---

## Phase 1: Template Engine Foundation ✅

**Status**: COMPLETE

### Files Created/Modified

| File | Status | Description |
|------|--------|-------------|
| `fortify-core/src/templates.rs` | ✅ Created | Template engine with placeholder substitution |
| `fortify-core/src/lib.rs` | ✅ Modified | Added `pub mod templates;` export |

### Components Implemented

1. **TemplateType Enum** - All 10 template types (Gate, Captcha, Error, Burned, Demoted, Maintenance, Recovery, Retiring, Busy, Index)

2. **Static Template Loading** - `include_str!()` for compile-time embedding of all HTML templates

3. **TemplateEngine Struct**
   - `render(TemplateType, vars)` - Render template with HashMap substitution
   - `render_string(template, vars)` - Render arbitrary string
   - `render_with_branding(type, branding, extra_vars)` - Convenience method
   - `with_custom_css(css)` - CSS injection support

4. **BrandingVars Struct** - Common branding variables:
   - `service_name`, `primary_color`, `secondary_color`, `tertiary_color`
   - `footer_branding`, `branding_injection`

5. **PrerenderedCaptchaPage Struct** - Pre-built CAPTCHA pages with:
   - `captcha_id` for answer verification
   - `html` complete page ready to serve
   - `generated_at` for staleness checks
   - `is_stale(max_age_secs)` validation

### Test Results

```
running 9 tests
test templates::tests::test_custom_css_injection ... ok
test templates::tests::test_placeholder_substitution ... ok
test templates::tests::test_branding_vars_to_hashmap ... ok
test templates::tests::test_nested_braces_handled ... ok
test templates::tests::test_template_loading ... ok
test templates::tests::test_prerendered_captcha_page ... ok
test templates::tests::test_template_type_all ... ok
test templates::tests::test_unmatched_placeholder_preserved ... ok
test templates::tests::test_template_type_filename ... ok

test result: ok. 9 passed; 0 failed
```

---

## Phase 2: Wire Static Templates ✅

**Status**: COMPLETE

### Objective
Replace inline HTML in `fortify-gate/src/server.rs` with template engine calls.

### Files Modified

| File | Change |
|------|--------|
| `fortify-gate/src/server.rs` | Added template imports, replaced `serve_landing_page()` |
| `assets/html/busy.html` | Created new template with CSS-only 20-second delay |

### Key Changes

1. **Import template types** in server.rs:
   ```rust
   use fortify_core::templates::{BrandingVars, TemplateEngine, TemplateType};
   ```

2. **Replace `serve_landing_page()`** - 120+ lines inline HTML → 6 lines:
   ```rust
   fn serve_landing_page(_gate: Arc<Gate>) -> Response<BoxBody> {
       let engine = TemplateEngine::new();
       let branding = BrandingVars::default();
       let html = engine.render_with_branding(TemplateType::Gate, &branding, None);
       // ... response builder
   }
   ```

3. **Add busy.html template** with CSS-only delay button (NO JavaScript):
   - Uses `animation: enableButton 20s forwards;` 
   - `pointer-events: none` until animation completes
   - Citadel/gold theme consistent with other templates

### Remaining Work (Future Commits)
- `serve_demoted_page()` - Complex, embeds dynamic CAPTCHA
- `serve_cookie_blocked_page()` - Error template
- `styled_error_response()` - Error template
- Success/verified page - Needs new template

---

## Phase 3: Pre-Rendered CAPTCHA Pages ⏳

| File | Function | Template |
|------|----------|----------|
| `fortify-gate/src/server.rs` | `serve_landing_page()` | Gate |
| `fortify-gate/src/server.rs` | `serve_demoted_page()` | Demoted |
| `fortify-gate/src/server.rs` | `serve_captcha_challenge()` | Captcha |
| `fortify-gate/src/server.rs` | `serve_error_page()` | Error |
| `fortify-gate/src/server.rs` | `serve_burned_page()` | Burned |
| `fortify-http/src/tarpit.rs` | `serve_busy_page()` | Busy |

---

## Phase 3: Pre-Rendered CAPTCHA Pages ⏳

**Status**: NOT STARTED

### Objective
Extend `CaptchaPoolManager` to store complete HTML pages, not just image data.

### Changes Required

1. Add `html_page: String` field to `PregenCaptcha` struct
2. Generate full HTML at pool replenishment time
3. Serve pre-rendered pages instead of assembling at request time

---

## Phase 4: Pool Expansion & Replenishment ⏳

**Status**: NOT STARTED

### Current Configuration
```toml
[captcha.pregen]
target_pool_size = 500
min_pool_size = 100
max_pool_size = 1000
batch_size = 10
```

### Proposed Configuration
```toml
[captcha.pregen]
target_pool_size = 25000
min_pool_size = 5000
max_pool_size = 50000
batch_size = 100
replenish_batch = 1000
```

### Rationale
- 50K CAPTCHAs = ~300MB storage (acceptable)
- At 100 req/sec attack, 50K pool lasts ~8 minutes
- Aggressive replenishment during quiet periods

---

## Phase 5: Edge Caching Preparation ⏳

**Status**: NOT STARTED

### Objective
Document nginx configuration for RAM-based CAPTCHA serving.

### Concept
```nginx
# Pre-load CAPTCHA pool into RAM
proxy_cache_path /dev/shm/captcha_cache levels=1:2 keys_zone=captcha:10m;

location /captcha/ {
    proxy_cache captcha;
    proxy_pass http://fortify_backend;
}
```

---

## References

- [UCAPTCHA Paper](https://example.com) - Tor-aware CAPTCHA research
- [Sprint 06: CAPTCHA Bug Sprint](06-CAPTCHA-BUG-SPRINT.md)
- [Sprint 07: Branding HTML Sprint](07-BRANDING-HTML-SPRINT.md)
