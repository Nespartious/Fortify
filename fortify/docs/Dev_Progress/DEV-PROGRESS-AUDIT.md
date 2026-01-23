# Dev_Progress Conflict Audit & Recommended Order

**Generated:** 2025-01-23  
**Last Updated:** 2026-01-23  
**Purpose:** Identify overlapping/conflicting tasks across sprint documents and recommend a working order

---

## Executive Summary

After reviewing all sprint documents and completing work on January 23-24, 2026:

### Completed This Session ✅
- **Sprint 05** - Security Status Bug: Hysteresis and decay fixes (ARCHIVED)
- **Sprint 06** - CAPTCHA Bug: Validation for all challenge types (ARCHIVED)
- **Sprint 02** - Panic Audit: Phases 2-3 complete (no unsafe patterns found)
- **Sprint 07** - Merged into Sprint 15 (ARCHIVED)
- **Sprint 08** - Variable Audit: Merged into Sprint 15
- **Sprint 13 Phase 1B** - Merged into Sprint 15
- **Sprint 14** - TUI & Control Panel Alignment: All 5 phases complete (ARCHIVED)
- **Sprint 03** - CI Quality: Automatic workflow triggers (PR #37 merged)
- **Sprint 04** - TUI Completion: Deployment wizard (PR #36 merged)

### Current State
- **5 Active Sprints** in Dev_Progress/
- **10 Archived Sprints** in Dev_Progress/archive/
- **Conflicts resolved** - Branding/Config work consolidated in Sprint 15
- **Sprint 15** - Branding & Config Propagation: 🟡 IN PROGRESS

### Next Priority
- **Sprint 15** - [15-BRANDING-CONFIG-PROPAGATION-SPRINT.md](15-BRANDING-CONFIG-PROPAGATION-SPRINT.md)
  - Fix branding propagation from TUI to Gate/HTTP
  - Remove deprecated config fields (tertiary_color, custom_css, logo, audio)
  - Ensure all templates use configurable values

---

## Conflict Analysis

### 🔴 CONFLICT 1: Branding Settings Implementation

**Sprints Involved:**
- [07-BRANDING-HTML-SPRINT.md](07-BRANDING-HTML-SPRINT.md)
- [13-COMBINED-CAPTCHA-LANDING-SPRINT.md](13-COMBINED-CAPTCHA-LANDING-SPRINT.md) (Phase 1B)
- [14-TUI-CP_ALIGNMENT-SPRINT.md](14-TUI-CP_ALIGNMENT-SPRINT.md) (Phases 1-2)

| Source | Scope | Status |
|--------|-------|--------|
| Sprint 07 | Add secondary/tertiary colors to BrandingConfig, update TUI & scripts | ⬜ Not Started |
| Sprint 13 Phase 1B | Extended branding vars (10+ fields), config file support | ⬜ Not Started |
| Sprint 14 Phases 1-2 | TUI + Control Panel branding UI, same fields | ⬜ Not Started |

**Diff Analysis:**
- **Sprint 07** focuses on extending BrandingConfig struct + HTML templates
- **Sprint 13** defines FULL branding requirements (most comprehensive)
- **Sprint 14** focuses on UI surfaces (TUI + Control Panel)

**Recommendation:** 
- ❌ **Remove from:** Sprint 07 (older, less complete)
- ✅ **Keep in:** Sprint 14 (most comprehensive, focuses on UI alignment)
- ⚡ **Merge into Sprint 14:** Sprint 13 Phase 1B branding requirements (the extended variable list)

---

### 🔴 CONFLICT 2: CAPTCHA Settings & Type Configuration

**Sprints Involved:**
- [06-CAPTCHA-BUG-SPRINT.md](06-CAPTCHA-BUG-SPRINT.md)
- [11-STATIC-CAPTCHA-TEMPLATES-SPRINT.md](11-STATIC-CAPTCHA-TEMPLATES-SPRINT.md)
- [13-COMBINED-CAPTCHA-LANDING-SPRINT.md](13-COMBINED-CAPTCHA-LANDING-SPRINT.md)
- [14-TUI-CP_ALIGNMENT-SPRINT.md](14-TUI-CP_ALIGNMENT-SPRINT.md) (Phases 3-4)

| Source | Scope | Status |
|--------|-------|--------|
| Sprint 06 | Fix unsolvable CAPTCHA bug (generation validation) | ⬜ Not Started |
| Sprint 11 | Template engine + pre-rendered CAPTCHA pages | ✅ Phase 1-2 Complete |
| Sprint 13 | Combined landing+captcha page, pool expansion | ⬜ Planning |
| Sprint 14 | Expose CAPTCHA type settings in TUI/CP | ⬜ Not Started |

**Diff Analysis:**
- **Sprint 06** is a BUG FIX - validates CAPTCHA generation (independent)
- **Sprint 11** is INFRASTRUCTURE - template engine (mostly complete)
- **Sprint 13** is OPTIMIZATION - combined page serving (depends on 11)
- **Sprint 14** is UI - expose settings (depends on types existing)

**Recommendation:**
- ✅ **Keep all as separate sprints** - they address different layers
- ⚡ **Working order:** 06 → 11 → 13 → 14 (dependency chain)
- 📝 **Note:** Sprint 06 should be done FIRST to ensure pool doesn't generate bad CAPTCHAs

---

### 🔴 CONFLICT 3: Security Thresholds Configuration

**Sprints Involved:**
- [05-SECURITY-STATUS-BUG-SPRINT.md](05-SECURITY-STATUS-BUG-SPRINT.md)
- [08-VARIABLE-AUDIT.md](08-VARIABLE-AUDIT.md)
- [14-TUI-CP_ALIGNMENT-SPRINT.md](14-TUI-CP_ALIGNMENT-SPRINT.md)

| Source | Scope | Status |
|--------|-------|--------|
| Sprint 05 | Fix status degradation + hysteresis logic | ⬜ Not Started |
| Sprint 08 | Audit ALL variables including hardcoded thresholds | ⬜ Not Started |
| Sprint 14 | Expose thresholds in UI | ⬜ Not Started |

**Diff Analysis:**
- **Sprint 05** fixes the BUG (status never degrades from Attack)
- **Sprint 08** AUDITS variables (documenting what exists)
- **Sprint 14** exposes settings in UI (making configurable)

**Recommendation:**
- ✅ **Keep Sprint 05** - must fix the bug first
- ❌ **Defer Sprint 08** - becomes redundant once Sprint 14 aligns everything
- ✅ **Keep Sprint 14** - comprehensive UI alignment includes threshold exposure
- ⚡ **Working order:** 05 → 14 → (08 becomes optional verification)

---

### 🔴 CONFLICT 4: Template System & HTML Updates

**Sprints Involved:**
- [07-BRANDING-HTML-SPRINT.md](07-BRANDING-HTML-SPRINT.md) (Phase 3)
- [11-STATIC-CAPTCHA-TEMPLATES-SPRINT.md](11-STATIC-CAPTCHA-TEMPLATES-SPRINT.md)
- [12-TEMPLATE-MIGRATION-SPRINT.md](12-TEMPLATE-MIGRATION-SPRINT.md)

| Source | Scope | Status |
|--------|-------|--------|
| Sprint 07 | Template variable injection, CSS variable system | ⬜ Not Started |
| Sprint 11 | Template engine with placeholder substitution | ✅ Complete |
| Sprint 12 | Migration of ALL inline HTML to templates | ✅ Complete |

**Diff Analysis:**
- **Sprint 07** describes implementing what Sprint 11+12 already did
- **Sprint 11** created the TemplateEngine with `{{PLACEHOLDER}}` substitution
- **Sprint 12** migrated all inline HTML to use templates

**Recommendation:**
- ❌ **Archive:** Sprint 07 Phase 3 (HTML Template System) - already implemented in Sprint 11+12
- ✅ **Keep in Sprint 07:** Phase 1 (BrandingConfig extension) and Phase 2 (TUI/Script updates) BUT merge into Sprint 14
- 📝 **Result:** Sprint 07 becomes fully absorbed into Sprint 14

---

## Overlap Analysis (Non-Conflicting)

### 🟡 OVERLAP 1: Panic Audit & Hardening

**Sprints:**
- [02-PANIC-AUDIT-SPRINT.md](02-PANIC-AUDIT-SPRINT.md) (Phases 2-4)
- [10-HARDENING-SPRINT.md](10-HARDENING-SPRINT.md)

**Status:**
- Sprint 02 Phase 1: ✅ Complete (lock safety)
- Sprint 10: ✅ Complete (semaphore, 503, jitter)

**Match:** No conflict - Sprint 10 implemented what SECURITY-REVIEW-COMPARISON.md recommended as additions to Sprint 02. Sprint 02 Phases 2-4 (HTTP headers, token parsing, fuzzing) are still valid remaining work.

---

### 🟡 OVERLAP 2: CI/CD & Clippy

**Sprints:**
- [03-CI-QUALITY-SPRINT.md](03-CI-QUALITY-SPRINT.md)
- [archive/CLIPPY-SPRINT.md](archive/CLIPPY-SPRINT.md)

**Status:**
- CLIPPY-SPRINT: ✅ Complete (archived)
- Sprint 03 Task 2: ✅ Complete (references completed CLIPPY-SPRINT)

**Match:** No conflict - properly linked. Sprint 03 remaining tasks (enable automatic triggers, configure thresholds) are independent.

---

### 🟡 OVERLAP 3: TUI Completion & Settings Alignment

**Sprints:**
- [04-TUI-COMPLETION-SPRINT.md](04-TUI-COMPLETION-SPRINT.md)
- [14-TUI-CP_ALIGNMENT-SPRINT.md](14-TUI-CP_ALIGNMENT-SPRINT.md)

**Status:**
- Sprint 04: ⬜ 40% Complete (vanity, verification, status polling)
- Sprint 14: ⬜ Not Started (settings UI alignment)

**Match:** No direct conflict - Sprint 04 is about WORKFLOW completion (deployment wizard steps), Sprint 14 is about SETTINGS alignment (what's configurable where). They can be worked in parallel.

---

## Clean Sprints (No Conflicts)

| Sprint | Topic | Status | Notes |
|--------|-------|--------|-------|
| 02-PANIC-AUDIT-SPRINT | Error handling | Phase 1 ✅ | Phases 2-4 remain |
| 03-CI-QUALITY-SPRINT | CI/CD workflows | Partial ✅ | Enable triggers remains |
| 04-TUI-COMPLETION-SPRINT | TUI wizard | 40% ✅ | Independent work |
| 09-TECH-STACK-AUDIT | Tech debt review | ⬜ | Research/documentation |
| 10-HARDENING-SPRINT | Security hardening | ✅ Complete | Archive candidate |

---

## Reference Documents (Not Work Items)

| Document | Purpose |
|----------|---------|
| 08-VARIABLE-AUDIT.md | Audit template/checklist |
| SECURITY-REVIEW-COMPARISON.md | Gap analysis reference |

---

## Recommended Actions

### Archive These (Completed)

| Document | Reason |
|----------|--------|
| 10-HARDENING-SPRINT.md | ✅ All 4 tasks complete |
| 11-STATIC-CAPTCHA-TEMPLATES-SPRINT.md | ✅ Phases 1-2 complete, remaining phases moved to Sprint 13 |
| 12-TEMPLATE-MIGRATION-SPRINT.md | ✅ Complete (already in active, should move to archive) |

### Consolidate Into Sprint 14

| Document | What to Merge |
|----------|---------------|
| 07-BRANDING-HTML-SPRINT.md | Phases 1-2 (BrandingConfig, TUI updates) |
| 13-COMBINED-CAPTCHA-LANDING-SPRINT.md | Phase 1B (branding requirements only) |

### Keep As-Is

| Document | Reason |
|----------|--------|
| 02-PANIC-AUDIT-SPRINT.md | Phases 2-4 still needed |
| 03-CI-QUALITY-SPRINT.md | Independent CI work |
| 04-TUI-COMPLETION-SPRINT.md | Wizard completion |
| 05-SECURITY-STATUS-BUG-SPRINT.md | Bug fix needed |
| 06-CAPTCHA-BUG-SPRINT.md | Bug fix needed |
| 09-TECH-STACK-AUDIT.md | Research/documentation |
| 13-COMBINED-CAPTCHA-LANDING-SPRINT.md | Architecture optimization (keep Phase 2+) |
| 14-TUI-CP_ALIGNMENT-SPRINT.md | Primary alignment work |

---

## Recommended Working Order

### Priority 1: Bug Fixes (Before Beta) ✅ COMPLETE
```
1. ✅ 05-SECURITY-STATUS-BUG-SPRINT    (status never degrades) - ARCHIVED
2. ✅ 06-CAPTCHA-BUG-SPRINT            (5% unsolvable CAPTCHAs) - ARCHIVED
3. ✅ 02-PANIC-AUDIT Phases 2-3        (HTTP/token parsing safety) - No fixes needed
```

### Priority 2: Feature Completion (NEXT)
```
4. 🟡 04-TUI-COMPLETION-SPRINT         (finish deployment wizard) ← RECOMMENDED NEXT
5. ✅ 14-TUI-CP_ALIGNMENT-SPRINT       (settings parity) - COMPLETE, ARCHIVED
6. 🟡 13-COMBINED-CAPTCHA-LANDING      (DDoS resilience optimization)
```

### Priority 3: Quality & Polish
```
7. ⬜ 03-CI-QUALITY-SPRINT             (enable automatic triggers)
8. ⬜ 02-PANIC-AUDIT Phase 4           (fuzzing infrastructure)
9. ⬜ 09-TECH-STACK-AUDIT              (documentation/research)
```

---

## Summary Table

| Document | Action | Status | Notes |
|----------|--------|--------|-------|
| 02-PANIC-AUDIT-SPRINT | Keep | ✅ Phases 1-3 Complete | Phase 4 (fuzzing) deferred |
| 03-CI-QUALITY-SPRINT | Keep | 🟡 Partial | Enable triggers remaining |
| 04-TUI-COMPLETION-SPRINT | Keep | 🟡 40% | Wizard completion |
| 05-SECURITY-STATUS-BUG-SPRINT | **ARCHIVED** | ✅ Complete | Hysteresis + decay fixes |
| 06-CAPTCHA-BUG-SPRINT | **ARCHIVED** | ✅ Complete | Validation + stress tests |
| 07-BRANDING-HTML-SPRINT | **ARCHIVED** | ➡️ Merged | Into Sprint 14 |
| 08-VARIABLE-AUDIT | Defer | ⬜ Reference | Verification checklist |
| 09-TECH-STACK-AUDIT | Keep | ⬜ Not Started | Research/documentation |
| 10-HARDENING-SPRINT | **ARCHIVED** | ✅ Complete | - |
| 11-STATIC-CAPTCHA-TEMPLATES-SPRINT | **ARCHIVED** | ✅ Complete | - |
| 12-TEMPLATE-MIGRATION-SPRINT | **ARCHIVED** | ✅ Complete | - |
| 13-COMBINED-CAPTCHA-LANDING-SPRINT | Keep | 🟡 Planning | Phase 1B → Sprint 14 |
| 14-TUI-CP_ALIGNMENT-SPRINT | **ARCHIVED** | ✅ Complete | All 5 phases done |
| SECURITY-REVIEW-COMPARISON | Keep | 📖 Reference | - |

---

## Questions for You

~~1. **Sprint 07:** Confirm merge into Sprint 14 and archive?~~ ✅ Done  
~~2. **Sprint 08:** Keep as verification checklist?~~ Decision: Defer  
~~3. **Sprint 10, 11, 12:** Confirm move to archive?~~ ✅ Done  
~~4. **Sprint 13 Phase 1B:** Merge into Sprint 14?~~ ✅ Done

