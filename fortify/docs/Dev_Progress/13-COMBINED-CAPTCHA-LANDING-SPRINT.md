# Sprint 13: Combined CAPTCHA Landing Page

**Status**: 🔄 PLANNING  
**Branch**: `feature/combined-captcha-landing`  
**Started**: 2025-01-22  
**Depends On**: Sprint 12 (Template Migration)

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
| [10-HARDENING-SPRINT.md](10-HARDENING-SPRINT.md) | Original proposal (lines 750-780) |
| [11-STATIC-CAPTCHA-TEMPLATES-SPRINT.md](11-STATIC-CAPTCHA-TEMPLATES-SPRINT.md) | Template engine & pre-gen pool |
| [CAPTCHA-POOL-MULTI-TYPE-PLANNING.md](../planning/CAPTCHA-POOL-MULTI-TYPE-PLANNING.md) | Multi-type pool planning |

## Current State Analysis

### Already Implemented ✅
1. **CaptchaPoolManager** - Pre-generates 200 BmpText CAPTCHAs at startup
2. **Pool persistence** - Saved to disk (`captcha_pool.json`)
3. **CPU-aware generation** - Pauses at 70% CPU usage
4. **Template engine** - All pages use templates
5. **Pre-rendered HTML** concept in `PrerenderedCaptchaPage` struct

### Not Yet Implemented ❌
1. **Combined landing+captcha template** - Still 2 separate pages (gate.html, captcha.html)
2. **HTTP proxy serves CAPTCHA** - Still proxies all traffic to Gate
3. **Pre-rendered full pages** - Pool stores image data, not complete HTML
4. **Edge caching config** - No nginx config for CAPTCHA caching

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

### Current `PregenCaptcha` Struct
```rust
pub struct PregenCaptcha {
    pub text: String,
    pub image_data: Vec<u8>,
    pub created_at: u64,
}
```

### Proposed `PregenCaptchaPage` Struct
```rust
pub struct PregenCaptchaPage {
    pub session_id: String,          // Pre-assigned session ID
    pub answer: String,              // Expected answer
    pub html_page: String,           // Complete rendered HTML
    pub captcha_type: CaptchaType,   // Type of captcha
    pub created_at: u64,             // For rotation
}
```

### Files to Modify
| File | Change |
|------|--------|
| `fortify-gate/src/lib.rs` | Add `PregenCaptchaPage` struct |
| `fortify-gate/src/lib.rs` | Update pool to generate full pages |
| `fortify-core/src/templates.rs` | Add helper for page pre-rendering |

---

## Phase 3: HTTP Proxy Direct Serving

**Status**: NOT STARTED

### Objective
Modify `fortify-http` to serve the combined CAPTCHA page directly from the pre-gen pool, bypassing Gate for page loads.

### Current Flow
```rust
// fortify-http/src/lib.rs - THREAT PATH
if trust_tier.requires_gate() {
    // Proxy ALL requests to Gate
    proxy_to_gate(req, &gate_address, &gate_path).await
}
```

### Proposed Flow
```rust
// fortify-http/src/lib.rs - THREAT PATH
if trust_tier.requires_gate() {
    // For landing page requests, serve pre-gen CAPTCHA directly
    if gate_path == "/Fortify" || gate_path == "/Fortify/Portcullis" {
        // Fetch pre-rendered page from pool
        serve_pregen_captcha_page(&pool).await
    } else {
        // Only proxy /verify and other dynamic routes to Gate
        proxy_to_gate(req, &gate_address, &gate_path).await
    }
}
```

### Implementation Considerations
1. **Pool access from HTTP proxy** - Need to share pool or API endpoint
2. **Session registration** - Pre-assigned session IDs must be registered with Gate
3. **Fallback** - If pool empty, fall back to Gate proxy
4. **Rate limiting** - Still apply rate limiting before serving pages

### Option A: Shared Pool via Controller API
```
HTTP Proxy → GET /api/captcha-page → Controller → Pool → Pre-rendered page
```

### Option B: Local Pool in HTTP Proxy
- HTTP Proxy maintains own copy of pre-gen pool
- Synced periodically from Controller
- Zero network latency for page serving

### Option C: Redis/Shared Memory
- Pre-gen pages stored in shared memory (e.g., /dev/shm)
- All services read from same location
- Highest performance, most complex

**Recommendation**: Option A (Controller API) for simplicity, migrate to Option C for production.

---

## Phase 4: Session Pre-Registration

**Status**: NOT STARTED

### Objective
When a pre-rendered page is taken from the pool, register its session ID with Gate so verification will work.

### Current Issue
Pre-rendered pages have session IDs embedded, but Gate doesn't know about them until verification attempt.

### Solution
```rust
// When page is served from pool:
1. Take page from pre-gen pool
2. Register session with Gate via API call
3. Serve page to user
4. User submits → Gate already has session → Verification works
```

### API Endpoint (New)
```
POST /gate/register-session
{
    "session_id": "uuid",
    "captcha_answer": "ABC123",
    "captcha_type": "bmptext",
    "is_threat": false,
    "captchas_remaining": 1
}
```

---

## Phase 5: Demoted User Flow

**Status**: NOT STARTED

### Objective
Ensure demoted users still see the "Hold Position" page and solve 2 different CAPTCHAs.

### Special Handling
- Demoted users should NOT get the combined page
- They get: demoted.html → (click Resume) → captcha page 1 → captcha page 2
- This is already implemented, just need to preserve it

### No Changes Needed
The `X-Fortify-Demoted` header logic remains unchanged.

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

**Recommendation**: Start after current demoted/captcha verification bugs are fully resolved and tested.

**Rationale**:
1. Current sprint (12) has residual bugs in demoted flow
2. Verification logic must be solid before adding complexity
3. Combined page is an optimization, not a bug fix
4. Better to have working 2-page flow than broken 1-page flow

**Trigger to Start Sprint 13**:
- [x] Demoted user sees Hold Position page
- [x] 2 different captcha types work for threat sessions
- [ ] Captcha verification succeeds (all types) ← Testing
- [ ] End-to-end flow tested and working ← Testing
- [ ] Sprint 12 doc archived

---

## Progress Log

### 2025-01-22
- Created Sprint 13 planning document
- Analyzed current vs proposed architecture
- Identified 7 implementation phases
- Added Phase 1B: Branding System Requirements
- Documented all branding variables and defaults
- Added branding success criteria
- Estimated ~19 hours total effort
- Recommendation: Start after current bugs resolved
