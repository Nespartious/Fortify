# Sprint 13: Combined CAPTCHA Landing Page

**Status**: 🔄 PLANNING (Updated 2026-01-25)  
**Branch**: `feature/combined-captcha-landing`  
**Started**: 2025-01-22  
**Depends On**: Sprint 12 (Template Migration) ✅ COMPLETE

## Overview

This sprint implements the **Combined CAPTCHA Landing Page** optimization identified in Sprint 10 (Hardening). The goal is to eliminate the 2-page hop for new users (landing page → separate captcha page) and serve a single combined page with an embedded pre-generated CAPTCHA, dramatically reducing Gate load.

## Problem Statement

### Current Architecture (Bottleneck)
```
User → HTTP Proxy → Gate → Landing page (gate.html) → User clicks "Request Entry"
                         → Gate → Generate CAPTCHA → Response (captcha.html)
                         → Gate → Verify submission
```

**Issues:**
1. **Two page loads** for new users (landing + captcha)
2. **Gate is the bottleneck** - attackers can exhaust Gate capacity
3. **CPU-bound CAPTCHA generation** at request time (even with pool)
4. **Legitimate users blocked** during attacks when Gate is overwhelmed

### Proposed Architecture (Optimized)
```
User → HTTP Proxy → Cached combined page (landing + captcha from pre-gen pool)
                         └─► /verify only → Gate (only verification needs processing)
```

**Benefits:**
- **97% Gate load reduction** - Only `/verify` hits Gate
- **Single page load** for new users (faster UX)
- **Pre-cached responses** - Near-zero CPU under DDoS
- **Edge-cacheable** - nginx can serve at 100K+ req/sec

## Related Documentation

| Document | Relevance |
|----------|-----------|
| [10-HARDENING-SPRINT.md](archive/10-HARDENING-SPRINT.md) | Original proposal |
| [11-STATIC-CAPTCHA-TEMPLATES-SPRINT.md](archive/11-STATIC-CAPTCHA-TEMPLATES-SPRINT.md) | Template engine & pre-gen pool |

---

## Current State Analysis (Updated 2026-01-25)

### Already Implemented ✅

1. **Template Engine** (`fortify-core/src/templates.rs`)
   - `TemplateEngine` with `TemplateType::Gate` and `TemplateType::Captcha` variants
   - Full branding variable support (`{{SERVICE_NAME}}`, `{{PRIMARY_COLOR}}`, etc.)
   - `PrerenderedCaptchaPage` struct exists - generates complete HTML with embedded CAPTCHA image

2. **Multi-Type CAPTCHA System** (`fortify-gate/src/captcha_types.rs`, `captcha_html.rs`)
   - 4 CAPTCHA types: BmpText, Emoji, Rotation, Silhouette
   - CSS-only rendering (Tor Browser Safest compatible)
   - Dynamic HTML generation per type

3. **CaptchaPoolManager** (`fortify-orchestrator/src/lib.rs`)
   - Pre-generates CAPTCHAs at startup
   - `PregenCaptcha` struct: `{ id, answer, image_data, generated_at, difficulty }`
   - Pool persistence to disk (`captcha_pool.json`)
   - CPU-aware generation (pauses at 70% CPU)

4. **Branding Propagation** (Sprints 14-16)
   - TUI → Controller → Gate branding sync
   - Runtime hot reload of branding variables
   - All templates use branding CSS variables

5. **HTML Templates** (`assets/html/`)
   - `gate.html` - Landing page with "Request Entry" button
   - `captcha.html` - CAPTCHA form page (uses `{{CAPTCHA_IMAGE_URL}}`)
   - Both have full branding integration

### NOT Yet Implemented ❌

1. **Combined `gate-challenge.html` Template**
   - Currently: 2 pages - `gate.html` → `captcha.html`
   - Need: 1 page with embedded CAPTCHA

2. **HTTP Proxy Direct CAPTCHA Serving**
   - Currently: HTTP proxy → Gate for ALL requests
   - Need: HTTP proxy serves pre-rendered CAPTCHA pages directly

3. **Pre-rendered Full HTML Pages in Pool**
   - Currently: Pool stores raw image bytes (`Vec<u8>`)
   - Need: Pool stores complete HTML pages (use `PrerenderedCaptchaPage`)

4. **Session Pre-Registration**
   - Currently: Session created when user hits Gate
   - Need: Session ID embedded in pre-rendered page, registered with Gate

---

## Phase 1: Combined Template Design

**Status**: NOT STARTED

### Objective
Create a single HTML template that combines the gate landing page with an embedded CAPTCHA challenge.

### New Template: `gate-challenge.html`

```
┌─────────────────────────────────────────┐
│            🏰 FORTIFY                   │
│         Entry Verification              │
├─────────────────────────────────────────┤
│                                         │
│  Welcome to the protected citadel.      │
│  Complete verification to enter.        │
│                                         │
│  ┌─────────────────────────────────┐   │
│  │  [CAPTCHA IMAGE - base64 data]  │   │
│  └─────────────────────────────────┘   │
│                                         │
│  Enter the code: [_____________]        │
│                                         │
│        [ VERIFY & ENTER ]               │
│                                         │
├─────────────────────────────────────────┤
│  Protected • Encrypted • No Scripts     │
└─────────────────────────────────────────┘
```

### Variables Required
- `{{CAPTCHA_IMAGE_DATA}}` - Base64 encoded BMP image
- `{{SESSION_ID}}` - Pre-assigned session ID
- `{{CAPTCHA_TYPE}}` - Type identifier (bmptext, emoji, etc.)
- Standard branding variables

---

## Phase 1B: Branding System Requirements

**Status**: ➡️ MERGED INTO SPRINT 14  
**See:** [14-TUI-CP_ALIGNMENT-SPRINT.md](14-TUI-CP_ALIGNMENT-SPRINT.md)

> **Note:** Phase 1B has been merged into Sprint 14 (TUI & Control Panel Alignment) which now owns all branding-related configuration alignment. This avoids duplication and consolidates branding work in one place.

### Summary (For Reference)
- Full branding variable set: service name, colors (primary/secondary/tertiary), logo, fonts, welcome message
- TUI and Control Panel must both support all branding settings
- Template placeholder system must have defaults for all variables
- See Sprint 14 for complete implementation tasks

---
| File | Purpose |
|------|---------|
| `assets/html/gate-challenge.html` | Combined landing + CAPTCHA template |
| `fortify-core/src/templates.rs` | Add `GateChallenge` variant |

---

## Phase 2: Pre-Rendered Page Pool

**Status**: NOT STARTED

### Objective
Extend `CaptchaPoolManager` to store complete HTML pages, not just image data.

### Current `PregenCaptcha` Struct (in fortify-orchestrator)
```rust
pub struct PregenCaptcha {
    pub id: String,           // Unique ID
    pub answer: String,       // Expected answer
    pub image_data: Vec<u8>,  // Raw BMP bytes
    pub generated_at: u64,    // Timestamp
    pub difficulty: u8,       // Difficulty level
}
```

### Existing `PrerenderedCaptchaPage` Struct (in fortify-core/templates.rs)
```rust
// NOTE: This already exists but is NOT used by the pool!
pub struct PrerenderedCaptchaPage {
    pub captcha_id: String,    // For answer verification
    pub html: String,          // Complete HTML page
    pub generated_at: u64,     // Staleness check
}
```

### Required Changes
The pool currently stores `PregenCaptcha` (raw image bytes). We need to either:
1. **Option A**: Change pool to store `PrerenderedCaptchaPage` directly
2. **Option B**: Keep `PregenCaptcha` and render HTML on-demand from HTTP proxy

**Recommendation**: Option A - store pre-rendered HTML pages in pool.

### Files to Modify
| File | Change |
|------|--------|
| `fortify-orchestrator/src/lib.rs` | Modify `CaptchaPoolManager` to use `PrerenderedCaptchaPage` |
| `fortify-orchestrator/src/lib.rs` | Add branding + template engine to pool generation |
| `fortify-core/src/templates.rs` | Add `TemplateType::GateChallenge` for combined template |

---

## Phase 3: HTTP Proxy Direct Serving

**Status**: NOT STARTED

### Objective
Modify `fortify-http` to serve the combined CAPTCHA page directly, bypassing Gate for initial page loads.

### Current Flow (from lib.rs)
```rust
// HTTP Proxy sends ALL unknown/threat users to Gate
if trust_tier.requires_gate() {
    proxy_to_gate(req, &gate_address, "/Fortify").await
}
```

### Proposed Flow
```rust
// HTTP Proxy serves pre-rendered CAPTCHA page directly
if trust_tier.requires_gate() && !is_demoted_user {
    if path == "/Fortify" || path == "/Fortify/Portcullis" {
        // Fetch pre-rendered combined page from Controller API
        serve_pregen_captcha_page().await
    } else if path.starts_with("/gate/verify") {
        // Only verification needs Gate
        proxy_to_gate(req, &gate_address, path).await
    }
}
```

### Communication Options

**Option A: Controller API (Recommended)**
```
HTTP Proxy → GET /api/captcha-page → Controller → Pool → Pre-rendered HTML
```
- Pros: Simple, centralized pool management
- Cons: Extra network hop

**Option B: Shared File System**
```
Orchestrator writes: /dev/shm/fortify/captcha-pages/
HTTP Proxy reads: /dev/shm/fortify/captcha-pages/
```
- Pros: Zero network latency
- Cons: File system coordination

**Option C: HTTP Proxy Local Pool**
- HTTP Proxy syncs pool from Controller on startup
- Generates its own pre-rendered pages
- Cons: Duplicated logic, memory usage

**Recommendation**: Start with Option A, optimize to B if needed.

---

## Phase 4: Session Pre-Registration

**Status**: NOT STARTED

### Objective
Pre-rendered pages have session IDs embedded. Gate must know about these sessions before verification.

### Solution: Lazy Registration
When user submits `/gate/verify`:
1. Gate checks if session exists
2. If not, create new session from CAPTCHA ID
3. Verify answer against pool's expected answer

### Alternative: Eager Registration
When page is served:
1. HTTP Proxy tells Controller "session X is now active"
2. Controller registers session with Gate
3. Extra network call, but cleaner

### API Endpoint (New)
```
POST /api/register-captcha-session
{
    "captcha_id": "uuid",
    "session_id": "uuid"  // Same as captcha_id or derived
}
```

---

## Phase 5: Demoted User Flow

**Status**: ALREADY HANDLED ✅

### Current Implementation
Demoted users are detected via `fortify_demoted` cookie and `X-Fortify-Demoted` header:
- They ALWAYS get redirected to `/Fortify` landing (not combined page)
- They see the "Hold Position" demoted.html page
- They must solve 2 CAPTCHAs (already implemented)

### No Changes Needed
The demoted user flow should remain separate from the combined page optimization.

---

## Phase 6: Pool Sizing & Rotation

**Status**: NOT STARTED

### Current Configuration
```
target_pool_size = 200
min_pool_size = 50
```

### Proposed Configuration
```toml
[captcha.pregen]
# Pre-rendered pages (complete HTML with embedded CAPTCHA)
page_pool_size = 1000        # 1000 ready-to-serve pages
page_min_pool = 200          # Minimum before aggressive replenish
page_max_age_secs = 300      # Max 5 minutes before rotation

# Raw captchas (for dynamic generation fallback)
raw_pool_size = 500
raw_min_pool = 100
```

### Memory Estimate
- Average page size: ~15KB
- 1000 pages = ~15MB RAM
- Acceptable for production

---

## Phase 7: Edge Caching (Optional)

**Status**: NOT STARTED

### Objective
Document nginx configuration for serving pre-rendered pages from RAM.

### Nginx Config Example
```nginx
# RAM-based cache for CAPTCHA pages
proxy_cache_path /dev/shm/fortify_cache levels=1:2 
    keys_zone=captcha:10m max_size=100m inactive=5m;

location /Fortify {
    # Try to serve from pre-gen pool first
    proxy_cache captcha;
    proxy_cache_valid 200 1m;
    proxy_cache_key $uri$is_args$args;
    proxy_pass http://fortify_http;
}
```

---

## Success Criteria

- [ ] Single page load for new users (landing + captcha combined)
- [ ] Gate only processes `/verify` requests (not page loads)
- [ ] 90%+ reduction in Gate CPU under normal load
- [ ] Pool replenishes automatically during quiet periods
- [ ] Demoted users still see Hold Position → 2 captcha flow
- [ ] **Branding**: No `{{PLACEHOLDER}}` text visible on any page
- [ ] **Branding**: Default theme shows citadel/gold colors correctly
- [ ] **Branding**: Operator-configured values display properly
- [ ] Build with zero errors and warnings

---

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Pool exhaustion under attack | Fallback to Gate proxy |
| Session ID collision | Use UUIDv7 with timestamp |
| Stale captchas | 5-minute rotation, validation on verify |
| Memory pressure | Configurable pool size, monitoring |

---

## Implementation Order

1. **Phase 1**: Create combined template (lowest risk, visual only)
2. **Phase 4**: Session pre-registration API (required for serving)
3. **Phase 2**: Pre-rendered page pool (build on existing pool)
4. **Phase 3**: HTTP proxy direct serving (the big change)
5. **Phase 5**: Verify demoted flow still works
6. **Phase 6**: Tune pool sizing based on testing
7. **Phase 7**: Optional edge caching documentation

---

## Estimated Effort

| Phase | Effort | Priority |
|-------|--------|----------|
| Phase 1 | 2 hours | HIGH |
| Phase 1B | 3 hours | HIGH |
| Phase 2 | 4 hours | HIGH |
| Phase 3 | 6 hours | HIGH |
| Phase 4 | 2 hours | HIGH |
| Phase 5 | 1 hour | MEDIUM |
| Phase 6 | 2 hours | MEDIUM |
| Phase 7 | 2 hours | LOW |

**Total**: ~22 hours

---

## Dependencies

- Sprint 12 complete (Template Migration) ✅
- `CaptchaPoolManager` operational ✅
- Pre-generated CAPTCHA pool working ✅
- Branding variables in `BrandingVars` ✅

---

## Decision: When to Start

**Recommendation**: Ready to start. Prerequisites are complete.

**Rationale**:
1. Branding propagation working (Sprints 14-17 complete)
2. Multi-type CAPTCHA system working (Emoji, Rotation, Silhouette, BmpText)
3. Template engine with full branding support
4. Backend redirect fix merged (Sprint 18)

**Prerequisites ✅**:
- [x] Demoted user sees Hold Position page
- [x] 2 different captcha types work for threat sessions
- [x] Captcha verification succeeds (all types)
- [x] End-to-end flow tested and working
- [x] Branding propagation complete (Sprints 14-17)
- [x] Backend redirect passthrough working (Sprint 18)

---

## Progress Log

### 2026-01-25
- **Updated document** with accurate current state
- Verified existing implementations:
  - `PrerenderedCaptchaPage` exists in templates.rs (not yet used by pool)
  - `CaptchaPoolManager` exists with `PregenCaptcha` (raw bytes, not HTML)
  - Multi-type CAPTCHA HTML generation in captcha_html.rs
- Updated Phase 2 to reflect existing vs needed structs
- Updated Phase 3 with clearer HTTP proxy integration options
- Marked Phase 5 (Demoted Flow) as already handled
- Marked all prerequisites as complete

### 2025-01-22
- Created Sprint 13 planning document
- Analyzed current vs proposed architecture
- Identified 7 implementation phases
- Added Phase 1B: Branding System Requirements
- Documented all branding variables and defaults
- Added branding success criteria
- Estimated ~22 hours total effort
- Recommendation: Start after current bugs resolved
