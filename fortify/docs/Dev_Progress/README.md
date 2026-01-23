# Dev Progress Master Tracker

**Last Updated:** January 23, 2026  
**Purpose:** Central tracking for all active development sprints

---

## ⚠️ AI Agent Instructions

**All AI agents working on this repository MUST follow the instructions in:**
- **[/.github/copilot-instructions.md](../../../.github/copilot-instructions.md)**

Key requirements:
1. **One sprint = One branch = One PR**
2. **Wait for ALL CI checks to pass** before considering work done
3. **Fix failures and retry** until all checks pass
4. **Provide summary report** with testing instructions at completion

---

## Quick Status Overview

| Sprint | Document | Status | Priority |
|--------|----------|--------|----------|
| **13** | [Combined CAPTCHA Landing](13-COMBINED-CAPTCHA-LANDING-SPRINT.md) | 📋 Planning | 🟡 MEDIUM |
| **15** | [Branding & Config Propagation](15-BRANDING-CONFIG-PROPAGATION-SPRINT.md) | 🟡 In Progress | 🔴 HIGH |

---

## Recently Archived (January 23, 2026)

| Sprint | Title | Merged PR |
|--------|-------|-----------|
| **02** | Panic Audit Phase 2 | PR #45 |
| **03** | CI/CD Quality Workflows | PR #37 |
| **04** | TUI Deployment Wizard | PR #36 |
| **09** | Tech Stack Audit | PR #38 |
| **16** | Traffic Tier Integration | PR #43 |
| **17** | Settings Hot Reload | PR #44 |

See [archive/](archive/) for completed sprint documentation.

---

## Active Sprint Details

### Sprint 15: Branding & Config Propagation
**File:** [15-BRANDING-CONFIG-PROPAGATION-SPRINT.md](15-BRANDING-CONFIG-PROPAGATION-SPRINT.md)  
**Status:** 🟡 In Progress  
**Priority:** 🔴 HIGH (Core Functionality)

**Goal:** Fix configuration propagation so TUI settings reach Gate/HTTP runtime.

**Completed:**
- ✅ TrafficTier enum and UI selectors (Sprint 16)
- ✅ Hot reload infrastructure (Sprint 17)

**Remaining:**
- ⬜ Fix `BrandingVars::default()` usages (8 locations)
- ⬜ Wire config to Gate/HTTP components
- ⬜ Test branding changes propagate to HTML

---

### Sprint 13: Combined CAPTCHA Landing Page
**File:** [13-COMBINED-CAPTCHA-LANDING-SPRINT.md](13-COMBINED-CAPTCHA-LANDING-SPRINT.md)  
**Status:** 📋 Planning  
**Priority:** 🟡 MEDIUM (Performance Optimization)

**Goal:** Eliminate 2-page hop for new users, serve combined landing+CAPTCHA page.

**Phases:**
- ⬜ Phase 1: Combined Template Design
- ⬜ Phase 2: Pool Expansion & Pre-rendering
- ⬜ Phase 3: HTTP Proxy Caching

**Blocked By:** None (can start anytime)

---

## Reference Documents

| Document | Purpose |
|----------|---------|
| [SECURITY-REVIEW-COMPARISON.md](SECURITY-REVIEW-COMPARISON.md) | Gap analysis vs external security review |
| [TEST-SESSION-SPRINT17.md](TEST-SESSION-SPRINT17.md) | Testing guide for Sprint 17 changes |

---

## Recommended Work Order

1. **Sprint 15** (Branding) - Fix config propagation to unblock user customization
2. **Sprint 13** (CAPTCHA Landing) - Performance optimization after core is stable

---

## Archive Contents

Located in [archive/](archive/):

| Sprint | Title | Completed |
|--------|-------|-----------|
| 01 | Timeout Strategy | Jan 2026 |
| 02 | Panic Audit | Jan 2026 |
| 03 | CI/CD Quality | Jan 2026 |
| 04 | TUI Completion | Jan 2026 |
| 05 | Security Status Bug | Jan 2026 |
| 06 | CAPTCHA Bug | Jan 2026 |
| 07 | Branding HTML | Jan 2026 |
| 08 | Variable Audit | Jan 2026 |
| 09 | Tech Stack Audit | Jan 2026 |
| 10 | Hardening | Jan 2026 |
| 11 | Static CAPTCHA Templates | Jan 2026 |
| 12 | Template Migration | Jan 2026 |
| 14 | TUI/CP Alignment | Jan 2026 |
| 16 | Tier Integration | Jan 2026 |
| 17 | Settings Hot Reload | Jan 2026 |
| - | Clippy Sprint | Jan 2026 |
