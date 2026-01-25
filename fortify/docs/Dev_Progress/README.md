# Dev Progress Master Tracker

**Last Updated:** January 25, 2026  
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
| **13** | [Combined CAPTCHA Landing](13-COMBINED-CAPTCHA-LANDING-SPRINT.md) | ✅ Complete (Phases 1-3) | 🟡 MEDIUM |
| **17** | [CAPTCHA Instruction Fix](17-CAPTCHA-INSTRUCTION-FIX-SPRINT.md) | ✅ Complete | 🟢 DONE |
| **18** | [Redirect Passthrough](archive/18-REDIRECT-PASSTHROUGH-SPRINT.md) | ✅ Complete | 🟢 DONE |

---

## Recently Archived (January 25, 2026)

| Sprint | Title | Merged PR |
|--------|-------|-----------|
| **18** | Redirect Passthrough (302 Fix) | PR #59 |
| **17** | CAPTCHA Instruction Fix | PR #58 |
| **13** | Combined CAPTCHA Landing (Phases 1-3) | PR #57 |
| **15** | Branding & Config Propagation | PR #46 |
| **02** | Panic Audit Phase 2 | PR #45 |
| **03** | CI/CD Quality Workflows | PR #37 |
| **04** | TUI Deployment Wizard | PR #36 |
| **09** | Tech Stack Audit | PR #38 |
| **16** | Traffic Tier Integration | PR #43 |

See [archive/](archive/) for completed sprint documentation.

---

## Active Sprint Details

### Sprint 13: Combined CAPTCHA Landing Page - Phase 4+
**File:** [13-COMBINED-CAPTCHA-LANDING-SPRINT.md](13-COMBINED-CAPTCHA-LANDING-SPRINT.md)  
**Status:** ✅ Phases 1-3 Complete (PR #57), Phase 4+ Future  
**Priority:** 🟡 MEDIUM (Performance Optimization)

**Goal:** Eliminate 2-page hop for new users, serve combined landing+CAPTCHA page.

**Completed Phases:**
- ✅ Phase 1: Combined Template (`gate-challenge.html`)
- ✅ Phase 2: `PrerenderedCaptchaPage` Update
- ✅ Phase 3: `CaptchaPoolManager` pre-rendering support

**Future Phases:**
- ⬜ Phase 4: HTTP Proxy Integration (serve pre-rendered pages directly)
- ⬜ Phase 5: Edge caching optimizations

---

## Reference Documents

| Document | Purpose |
|----------|---------|
| [SECURITY-REVIEW-COMPARISON.md](SECURITY-REVIEW-COMPARISON.md) | Gap analysis vs external security review |
| [TEST-SESSION-SPRINT17.md](TEST-SESSION-SPRINT17.md) | Testing guide for Sprint 17 changes |

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
| 18 | Redirect Passthrough | Jan 2026 |
| - | Clippy Sprint | Jan 2026 |
