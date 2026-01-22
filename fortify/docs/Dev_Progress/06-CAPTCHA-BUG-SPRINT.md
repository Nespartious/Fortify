# Sprint: Unsolvable CAPTCHA Bug Fix

**Sprint ID:** BUG-002  
**Priority:** 🔴 HIGH (User-Reported Bug)  
**GitHub Issue:** [#2](https://github.com/Nespartious/Fortify/issues/2)  
**Estimated Effort:** 2-3 days  
**Status:** ⬜ Not Started  
**Created:** January 22, 2026

---

## Problem Statement

Approximately 5% of CAPTCHAs served are unsolvable. User reports specific failure modes:
- **Emoji selection:** Required emoji not visible in options
- **Choose emotion:** Target emotion not shown in image options
- **Choose animal:** Target animal not present in selection

### Impact
- Users unable to verify despite being legitimate
- Frustration leads to abandonment
- False positives in threat detection (failed attempts counted)
- 5% failure rate = 1 in 20 users affected

---

## Root Cause Analysis

### Suspected Causes

1. **CAPTCHA Generation Logic Bugs**
   - Target answer not properly included in displayed options
   - Randomization may exclude the correct answer
   - Edge cases in shuffle/selection algorithms

2. **Pool Corruption**
   - Bad CAPTCHAs persisting in pool
   - No validation at generation time

3. **Emoji/Image Rendering**
   - Unicode emoji not rendering on certain browsers
   - Image paths broken or missing

---

## Investigation Plan

### Task 1: Audit CAPTCHA Generation Code
**Location:** `crates/fortify-orchestrator/src/lib.rs` (CaptchaPoolManager)  
**Files to check:**
- Pool generation logic
- Answer selection algorithm
- Option shuffling code

**Things to verify:**
```rust
// MUST verify these invariants:
// 1. correct_answer is ALWAYS in displayed_options
// 2. displayed_options.len() >= min_options
// 3. All emojis in options are renderable
// 4. All images in options exist
```

### Task 2: Analyze CAPTCHA Types
**Current types to audit:**
- Text/character CAPTCHAs
- Emoji selection CAPTCHAs
- Image grid CAPTCHAs (animal/emotion)
- Math CAPTCHAs

**For each type verify:**
```rust
struct CaptchaChallenge {
    challenge_type: CaptchaType,
    question: String,           // What user sees
    correct_answer: String,     // Expected answer
    displayed_options: Vec<..>, // Options shown
    // INVARIANT: correct_answer MUST be in displayed_options
}
```

### Task 3: Add Generation-Time Validation
```rust
impl CaptchaPoolManager {
    fn generate_captcha(&mut self) -> Result<Captcha, CaptchaError> {
        let captcha = self.create_captcha_challenge()?;
        
        // VALIDATION: Ensure captcha is solvable
        if !self.validate_captcha(&captcha) {
            tracing::warn!("Generated invalid CAPTCHA, regenerating");
            return self.generate_captcha(); // Retry
        }
        
        Ok(captcha)
    }
    
    fn validate_captcha(&self, captcha: &Captcha) -> bool {
        // 1. Check answer exists in options
        let answer_present = captcha.displayed_options
            .iter()
            .any(|opt| opt.value == captcha.correct_answer);
        
        // 2. Check minimum options present
        let enough_options = captcha.displayed_options.len() >= 4;
        
        // 3. Check no duplicate options
        let unique_options = captcha.displayed_options.len() == 
            captcha.displayed_options.iter()
                .map(|o| &o.value)
                .collect::<HashSet<_>>()
                .len();
        
        answer_present && enough_options && unique_options
    }
}
```

---

## Implementation Tasks

### Task 1: CAPTCHA Type Audit
**Status:** ⬜ Not Started  
**Estimated Time:** 2 hours

- [ ] List all CAPTCHA generation methods
- [ ] Trace each type's generation flow
- [ ] Document current invariant guarantees
- [ ] Identify where answer inclusion can fail

### Task 2: Add CAPTCHA Validation
**Status:** ⬜ Not Started  
**Estimated Time:** 2 hours

- [ ] Create `validate_captcha()` method
- [ ] Add validation call after each generation
- [ ] Add retry logic for invalid CAPTCHAs
- [ ] Log validation failures with details

### Task 3: Fix Emoji CAPTCHA Generation
**Status:** ⬜ Not Started  
**Estimated Time:** 1 hour

- [ ] Audit emoji selection algorithm
- [ ] Ensure correct emoji always in grid
- [ ] Filter to "safe" emoji subset (cross-browser compatible)
- [ ] Add emoji rendering test

### Task 4: Fix Image CAPTCHA Generation
**Status:** ⬜ Not Started  
**Estimated Time:** 1 hour

- [ ] Audit animal/emotion selection
- [ ] Verify image paths exist at generation time
- [ ] Ensure target image always included
- [ ] Add fallback for missing images

### Task 5: Pool Health Monitoring
**Status:** ⬜ Not Started  
**Estimated Time:** 1 hour

- [ ] Add pool validation sweep (check existing CAPTCHAs)
- [ ] Purge invalid CAPTCHAs from pool
- [ ] Add metrics for invalid CAPTCHA rate
- [ ] Alert if validation failure rate > 1%

### Task 6: Testing
**Status:** ⬜ Not Started  
**Estimated Time:** 2 hours

- [ ] Add unit tests for each CAPTCHA type
- [ ] Fuzz test CAPTCHA generation (1000+ samples)
- [ ] Verify 0% failure rate in tests
- [ ] Manual testing in browser

### Task 7: User Feedback Loop
**Status:** ⬜ Not Started  
**Estimated Time:** 30 minutes

- [ ] Add "Report Unsolvable" button (optional)
- [ ] Log unsolvable reports with CAPTCHA ID
- [ ] Allow CAPTCHA regeneration for user

---

## Code Investigation Starting Points

```bash
# Find CAPTCHA generation code
grep -rn "generate.*captcha\|create.*captcha" crates/ --include="*.rs"

# Find option selection code
grep -rn "shuffle\|random.*select\|pick.*option" crates/ --include="*.rs"

# Find emoji-related code
grep -rn "emoji\|Emoji" crates/ --include="*.rs"

# Check captcha types
grep -rn "CaptchaType\|CaptchaKind" crates/ --include="*.rs"
```

---

## Test Plan

### Automated Tests
```rust
#[test]
fn test_captcha_always_solvable() {
    let mut manager = CaptchaPoolManager::new(config);
    
    for _ in 0..1000 {
        let captcha = manager.generate_captcha().unwrap();
        
        // Answer must be in options
        assert!(
            captcha.displayed_options.iter().any(|o| o.value == captcha.correct_answer),
            "CAPTCHA {} has answer '{}' not in options {:?}",
            captcha.id, captcha.correct_answer, captcha.displayed_options
        );
        
        // Must have enough options
        assert!(captcha.displayed_options.len() >= 4);
        
        // Options must be unique
        let unique: HashSet<_> = captcha.displayed_options.iter()
            .map(|o| &o.value)
            .collect();
        assert_eq!(unique.len(), captcha.displayed_options.len());
    }
}
```

### Manual Browser Test
1. Access gate with CAPTCHA enabled
2. Solve 50 CAPTCHAs manually
3. Record any unsolvable ones with screenshots
4. Verify 0 unsolvable after fix

---

## Acceptance Criteria

- [ ] CAPTCHA generation validates answer presence
- [ ] Invalid CAPTCHAs are rejected and regenerated
- [ ] Fuzz test generates 10,000 CAPTCHAs with 0 failures
- [ ] Pool validation removes existing invalid CAPTCHAs
- [ ] Metrics track validation failure rate
- [ ] Manual testing shows 0% unsolvable rate
- [ ] GitHub issue #2 closed with fix reference

---

## References

- CAPTCHA generation: `crates/fortify-orchestrator/src/lib.rs`
- CAPTCHA types: `crates/fortify-gate/src/captcha_types.rs`
- CAPTCHA rendering: `crates/fortify-gate/src/captcha_html.rs`
- Image assets: `assets/images/captcha/`
