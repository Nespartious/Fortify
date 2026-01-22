# Sprint: CI/CD Quality Workflows

**Sprint ID:** QA-001  
**Priority:** 🟡 MEDIUM  
**Estimated Effort:** 1-2 days  
**Status:** ⬜ Not Started  
**Created:** January 22, 2026

---

## Objective

Enable and configure the new CI/CD quality workflows that were added to the repository. Remove lint suppressions and ensure all workflows run automatically on push/PR.

## Background

On January 22, 2026, 8 new workflows were added:
- `dependency-review.yml` - Check for vulnerable dependencies
- `mutation-testing.yml` - Validate test quality with cargo-mutants
- `fuzz-testing.yml` - Find edge cases with cargo-fuzz
- `sbom.yml` - Generate Software Bill of Materials
- `conventional-commits.yml` - Enforce commit message format
- `pr-size.yml` - Track and label PR sizes
- `doc-coverage.yml` - Check API documentation coverage
- `breaking-change.yml` - Detect semver violations

Currently all workflows are manual-only. This sprint enables automatic triggers.

---

## Implementation Tasks

### Task 1: Enable Automatic Triggers on Existing Workflows
**Status:** ⬜ Not Started  
**Estimated Time:** 30 min

**Files to Modify:**
- `.github/workflows/ci.yml`
- `.github/workflows/code-quality.yml`
- `.github/workflows/coverage.yml`
- `.github/workflows/security.yml`

**Change:** Uncomment or add push/PR triggers:
```yaml
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  workflow_dispatch:  # Keep manual trigger
```

**Sub-tasks:**
- [ ] Enable push/PR triggers on ci.yml
- [ ] Enable push/PR triggers on code-quality.yml
- [ ] Enable push/PR triggers on coverage.yml
- [ ] Enable push/PR triggers on security.yml

---

### Task 2: Remove Clippy Lint Suppressions
**Status:** ⬜ Not Started  
**Estimated Time:** 2-4 hours (depends on lint count)

**File:** `.github/workflows/ci.yml`

**Current State:** 54 clippy lints suppressed with `-A` flags.

**Action:** After completing CLIPPY-SPRINT.md fixes, remove suppressions:
```yaml
# BEFORE (suppressed)
env:
  CLIPPY_FLAGS: "-A clippy::cast_possible_truncation -A clippy::cast_sign_loss ..."

# AFTER (enforced)
env:
  CLIPPY_FLAGS: ""  # Or remove entirely
```

**Dependencies:** Requires completing [CLIPPY-SPRINT.md](CLIPPY-SPRINT.md) first.

**Sub-tasks:**
- [ ] Complete Phase 1 of CLIPPY-SPRINT (safety-critical casts)
- [ ] Remove corresponding `-A` flags from CI
- [ ] Verify CI passes
- [ ] Repeat for Phases 2-5

---

### Task 3: Configure Coverage Threshold
**Status:** ⬜ Not Started  
**Estimated Time:** 15 min

**File:** `.github/workflows/coverage.yml`

**Add:** Fail CI if coverage drops below threshold:
```yaml
- name: Check coverage threshold
  run: |
    COVERAGE=$(cat coverage.txt | grep "Total:" | awk '{print $2}')
    THRESHOLD=50
    if [ "$COVERAGE" -lt "$THRESHOLD" ]; then
      echo "Coverage $COVERAGE% is below threshold $THRESHOLD%"
      exit 1
    fi
```

**Sub-tasks:**
- [ ] Determine current coverage baseline
- [ ] Set initial threshold (suggest: 50%)
- [ ] Add threshold check to workflow
- [ ] Add coverage badge to README

---

### Task 4: Configure Dependency Review
**Status:** ⬜ Not Started  
**Estimated Time:** 15 min

**File:** `.github/workflows/dependency-review.yml`

**Verify Configuration:**
- [ ] Fails on high/critical vulnerabilities
- [ ] Checks license compliance
- [ ] Runs on all PRs

**Test:** Create a PR that adds a known-vulnerable dependency.

---

### Task 5: Configure Mutation Testing
**Status:** ⬜ Not Started  
**Estimated Time:** 30 min

**File:** `.github/workflows/mutation-testing.yml`

**Current:** Weekly schedule + manual trigger.

**Enhancements:**
- [ ] Set mutation score threshold (suggest: 30% initially)
- [ ] Add badge to README
- [ ] Configure to run on specific paths only (performance)

```yaml
- name: Check mutation score
  run: |
    SCORE=$(cargo mutants --score-only 2>&1 | tail -1)
    if [ "$SCORE" -lt 30 ]; then
      echo "Mutation score $SCORE% is below threshold"
    fi
```

---

### Task 6: Configure Fuzz Testing
**Status:** ⬜ Not Started  
**Estimated Time:** 1 hour

**File:** `.github/workflows/fuzz-testing.yml`

**Current:** Daily schedule + manual trigger.

**Actions:**
- [ ] Create initial fuzz targets in `fortify-http/fuzz/`
- [ ] Create fuzz targets in `fortify-core/fuzz/`
- [ ] Verify workflow can discover targets
- [ ] Configure crash artifact retention

**Fuzz Targets to Create:**
1. HTTP header parsing
2. Token/session parsing
3. Cookie parsing
4. Onion address validation

---

### Task 7: Configure SBOM Generation
**Status:** ⬜ Not Started  
**Estimated Time:** 15 min

**File:** `.github/workflows/sbom.yml`

**Verify:**
- [ ] Generates CycloneDX format SBOM
- [ ] Attaches to releases
- [ ] Uploads as artifact

---

### Task 8: Remove `|| true` Bypasses
**Status:** ⬜ Not Started  
**Estimated Time:** 30 min

**Problem:** Many workflow steps use `|| true` to suppress failures.

**Files to Audit:**
- All workflow files in `.github/workflows/`

**Command:**
```bash
grep -rn "|| true" .github/workflows/
```

**For Each Occurrence:**
- Determine if failure should actually fail the build
- Remove `|| true` or replace with `continue-on-error: true` for optional steps

---

### Task 9: Remove `continue-on-error: true` Where Inappropriate
**Status:** ⬜ Not Started  
**Estimated Time:** 30 min

**Problem:** Some critical security checks use `continue-on-error: true`.

**Command:**
```bash
grep -rn "continue-on-error: true" .github/workflows/
```

**Review Each:**
- Security audit should fail the build on high/critical
- Linting should fail the build
- Tests should fail the build

---

### Task 10: Add PR Size Labels
**Status:** ⬜ Not Started  
**Estimated Time:** 15 min

**File:** `.github/workflows/pr-size.yml`

**Create Labels:**
- `size/XS` (green)
- `size/S` (green)
- `size/M` (yellow)
- `size/L` (orange)
- `size/XL` (red)

**GitHub Command:**
```bash
gh label create "size/XS" --color "0E8A16" --description "< 50 lines"
gh label create "size/S" --color "0E8A16" --description "< 200 lines"
gh label create "size/M" --color "FBCA04" --description "< 400 lines"
gh label create "size/L" --color "D93F0B" --description "< 800 lines"
gh label create "size/XL" --color "B60205" --description "> 800 lines"
```

---

## Completion Checklist

- [ ] All workflows have automatic push/PR triggers
- [ ] Clippy lint suppressions removed (after CLIPPY-SPRINT)
- [ ] Coverage threshold enforced
- [ ] Dependency review configured
- [ ] Mutation testing configured
- [ ] Fuzz targets created
- [ ] SBOM verified
- [ ] `|| true` bypasses removed
- [ ] `continue-on-error` reviewed
- [ ] PR size labels created

---

## Validation

After completion, verify:
```bash
# Trigger CI manually
gh workflow run ci.yml

# Check all workflows pass
gh run list --limit 10
```

---

## References

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [cargo-mutants](https://mutants.rs/)
- [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- New workflows commit: `319632d`
