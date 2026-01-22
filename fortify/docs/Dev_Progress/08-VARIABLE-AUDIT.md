# Variable Implementation & Usage Audit

**Document ID:** AUDIT-001  
**Priority:** 🟡 MEDIUM (Quality Assurance)  
**Estimated Effort:** 3-5 days (ongoing)  
**Status:** ⬜ Not Started  
**Created:** January 22, 2026

---

## Objective

Build a comprehensive inventory of every configurable variable in Fortify and trace its usage through the codebase to verify:

1. Variable is used where intended
2. No orphaned/unused variables
3. Defaults are applied consistently
4. Validation is performed on input
5. Changes propagate correctly (TUI → Config → Runtime)

---

## Variable Categories

### 1. Branding Variables
| Variable | Location | TUI | Script | Default | Validation | Usage Verified |
|----------|----------|-----|--------|---------|------------|----------------|
| `service_name` | `BrandingConfig` | ✅ | ⬜ | "Protected Service" | ⬜ Max 100 | ⬜ |
| `description` | `BrandingConfig` | ✅ | ⬜ | "A Fortify-protected..." | ⬜ Max 100 | ⬜ |
| `logo_path` | `BrandingConfig` | ✅ | ⬜ | None | ⬜ Path exists | ⬜ |
| `primary_color` | `BrandingConfig` | ✅ | ⬜ | "#6B46C1" | ⬜ Hex format | ⬜ |
| `secondary_color` | `BrandingConfig` | ❌ NEW | ❌ NEW | TBD | ⬜ Hex format | ⬜ |
| `tertiary_color` | `BrandingConfig` | ❌ NEW | ❌ NEW | TBD | ⬜ Hex format | ⬜ |
| `welcome_message` | `BrandingConfig` | ✅ | ⬜ | "Please complete..." | ⬜ | ⬜ |
| `custom_css` | `BrandingConfig` | ✅ | ⬜ | None | ⬜ CSS valid | ⬜ |
| `logo_max_width` | `BrandingConfig` | ✅ | ⬜ | 256 | ⬜ Range | ⬜ |
| `logo_max_height` | `BrandingConfig` | ✅ | ⬜ | 256 | ⬜ Range | ⬜ |

### 2. CAPTCHA Variables
| Variable | Location | TUI | Script | Default | Validation | Usage Verified |
|----------|----------|-----|--------|---------|------------|----------------|
| `enabled` | `CaptchaConfig` | ✅ | ⬜ | true | ⬜ | ⬜ |
| `pool_size` | `CaptchaConfig` | ✅ | ⬜ | 500 | ⬜ Range | ⬜ |
| `min_pool_size` | `CaptchaConfig` | ✅ | ⬜ | 100 | ⬜ < pool_size | ⬜ |
| `max_pool_size` | `CaptchaConfig` | ✅ | ⬜ | 1000 | ⬜ > pool_size | ⬜ |
| `difficulty` | `CaptchaConfig` | ✅ | ⬜ | 5 | ⬜ 1-10 | ⬜ |
| `timeout_seconds` | `CaptchaConfig` | ✅ | ⬜ | 120 | ⬜ > 0 | ⬜ |
| `max_attempts` | `CaptchaConfig` | ✅ | ⬜ | 3 | ⬜ > 0 | ⬜ |
| `audio_enabled` | `CaptchaConfig` | ✅ | ⬜ | false | ⬜ | ⬜ |
| `rotation_percent` | `CaptchaConfig` | ✅ | ⬜ | 25 | ⬜ 0-100 | ⬜ |
| `rotation_interval_days` | `CaptchaConfig` | ✅ | ⬜ | 10 | ⬜ > 0 | ⬜ |

### 3. Threshold Variables
| Variable | Location | TUI | Script | Default | Validation | Usage Verified |
|----------|----------|-----|--------|---------|------------|----------------|
| `rate_limit_rpm` | `ThresholdConfig` | ✅ | ⬜ | 60 | ⬜ | ⬜ |
| `burst_multiplier` | `ThresholdConfig` | ✅ | ⬜ | 3.0 | ⬜ | ⬜ |
| `session_timeout_minutes` | `ThresholdConfig` | ✅ | ⬜ | 30 | ⬜ | ⬜ |
| `max_failed_captcha` | `ThresholdConfig` | ✅ | ⬜ | 5 | ⬜ | ⬜ |
| `proof_of_work_difficulty` | `ThresholdConfig` | ✅ | ⬜ | 4 | ⬜ | ⬜ |

### 4. Network Variables
| Variable | Location | TUI | Script | Default | Validation | Usage Verified |
|----------|----------|-----|--------|---------|------------|----------------|
| `listen_address` | Config | ✅ | ⬜ | "127.0.0.1" | ⬜ Valid IP | ⬜ |
| `listen_port` | Config | ✅ | ⬜ | 8080 | ⬜ Valid port | ⬜ |
| `gate_port` | Config | ✅ | ⬜ | 9999 | ⬜ Valid port | ⬜ |
| `orchestrator_port` | Config | ✅ | ⬜ | 9998 | ⬜ Valid port | ⬜ |
| `admin_port` | Config | ✅ | ⬜ | 9001 | ⬜ Valid port | ⬜ |
| `backend_url` | Config | ✅ | ⬜ | Required | ⬜ Valid URL | ⬜ |

### 5. Tor Variables
| Variable | Location | TUI | Script | Default | Validation | Usage Verified |
|----------|----------|-----|--------|---------|------------|----------------|
| `tor_control_port` | Config | ✅ | ⬜ | 9051 | ⬜ | ⬜ |
| `tor_control_password` | Config | ✅ | ⬜ | None | ⬜ | ⬜ |
| `onion_address` | Config | ✅ | ⬜ | Required | ⬜ .onion | ⬜ |
| `hidden_service_dir` | Config | ✅ | ⬜ | None | ⬜ Path | ⬜ |

### 6. Security Status Thresholds
| Variable | Location | TUI | Script | Default | Validation | Usage Verified |
|----------|----------|-----|--------|---------|------------|----------------|
| `attack_sessions_threshold` | ⬜ Hardcoded | ❌ | ❌ | 100 | N/A | ⬜ |
| `attack_unverified_threshold` | ⬜ Hardcoded | ❌ | ❌ | 500 | N/A | ⬜ |
| `attack_failed_captcha_threshold` | ⬜ Hardcoded | ❌ | ❌ | 20 | N/A | ⬜ |
| `warning_sessions_threshold` | ⬜ Hardcoded | ❌ | ❌ | 60 | N/A | ⬜ |
| `warning_unverified_threshold` | ⬜ Hardcoded | ❌ | ❌ | 300 | N/A | ⬜ |

---

## Audit Process

### Step 1: Variable Discovery
```bash
# Find all config struct definitions
grep -rn "pub struct.*Config" crates/ --include="*.rs"

# Find all field definitions in config structs
grep -rn "pub [a-z_]*:" crates/*/src/config.rs

# Find hardcoded thresholds
grep -rn "if.*>[[:space:]]*[0-9]" crates/ --include="*.rs" | grep -v test
```

### Step 2: Usage Tracing
For each variable, trace:
1. **Definition:** Where is it defined?
2. **Default:** What is the default value?
3. **Input:** Where can it be set? (TUI/Script/Config file)
4. **Validation:** Is input validated?
5. **Propagation:** How does it reach runtime code?
6. **Usage:** Where is it actually used?

### Step 3: Verification Checklist
For each variable:
- [ ] Default is sensible and documented
- [ ] TUI input works and saves correctly
- [ ] Script input works (if applicable)
- [ ] Config file loading works
- [ ] Validation rejects bad input
- [ ] Value reaches runtime code
- [ ] Changing value has expected effect
- [ ] Edge cases handled (min/max/empty)

---

## Tracing Template

### Variable: `{variable_name}`

**Definition:**
```
File: {file_path}
Line: {line_number}
Type: {rust_type}
```

**Default Value:** `{default_value}`

**Input Sources:**
| Source | File | Line | Status |
|--------|------|------|--------|
| TUI Wizard | | | ⬜ |
| TUI Settings | | | ⬜ |
| Config File | | | ⬜ |
| Install Script | | | ⬜ |
| CLI Argument | | | ⬜ |

**Validation:**
```
File: {file_path}
Line: {line_number}
Check: {validation_description}
```

**Usage Points:**
| File | Line | Purpose |
|------|------|---------|
| | | |

**Test Coverage:**
- [ ] Unit test exists
- [ ] Integration test exists
- [ ] Manual test documented

**Issues Found:**
- 

---

## Known Issues to Investigate

### 1. Hardcoded Security Thresholds
**Location:** `crates/fortify-tui/src/logging.rs:280-297`
```rust
self.level = if sessions_per_min > 100 || unverified_per_min > 500 || failed_captcha > 20 {
    SecurityLevel::Attack
```
**Issue:** These should be configurable via `ThresholdConfig`

### 2. Secondary/Tertiary Colors Missing
**Status:** Planned for FEATURE-001 sprint

### 3. Color Validation Missing
**Issue:** No validation that colors are valid hex format

### 4. Script Input Incomplete
**Issue:** Install script may not prompt for all configurable values

---

## Implementation Tasks

### Task 1: Complete Variable Inventory
**Status:** ⬜ Not Started

- [ ] Scan all config structs
- [ ] Document every field
- [ ] Identify hardcoded values that should be configurable

### Task 2: Trace Each Variable
**Status:** ⬜ Not Started

- [ ] Use template above for each variable
- [ ] Document input sources
- [ ] Document usage points

### Task 3: Identify Gaps
**Status:** ⬜ Not Started

- [ ] Variables not exposed in TUI
- [ ] Variables not exposed in scripts
- [ ] Missing validation
- [ ] Missing defaults

### Task 4: Create Test Cases
**Status:** ⬜ Not Started

- [ ] Test default values load correctly
- [ ] Test TUI changes persist
- [ ] Test validation rejects bad input
- [ ] Test runtime uses configured values

### Task 5: Document Findings
**Status:** ⬜ Not Started

- [ ] Create operator documentation for each variable
- [ ] Document recommended values
- [ ] Document security implications

---

## Related Documents

- [07-BRANDING-HTML-SPRINT.md](07-BRANDING-HTML-SPRINT.md) - Branding variable additions
- [05-SECURITY-STATUS-BUG-SPRINT.md](05-SECURITY-STATUS-BUG-SPRINT.md) - Security threshold issues
- [04-TUI-COMPLETION-SPRINT.md](04-TUI-COMPLETION-SPRINT.md) - TUI settings implementation
