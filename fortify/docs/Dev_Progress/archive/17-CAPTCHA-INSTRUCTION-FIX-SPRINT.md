# Sprint 17: CAPTCHA Instruction Text Fix

**Status:** 🔴 NOT STARTED  
**Priority:** HIGH - Affects user verification success rate  
**Branch:** `fix/sprint-17-captcha-instructions`  
**PR:** TBD

---

## Objective

Fix incorrect or missing instruction text in CAPTCHA rendering that causes user confusion and verification failures.

---

## Issues Identified

### Issue 1: Emoji CAPTCHA - Wrong Instruction Text (CRITICAL)

**File:** `crates/fortify-gate/src/captcha_html.rs`  
**Lines:** 750-751

**Current Code (BROKEN):**
```rust
CaptchaData::Emoji(challenge) => {
    let instruction = format!("Select all <strong>{}</strong>", challenge.target_category);
    render_emoji_captcha_with_message(session_id, challenge, is_threat, title, &instruction)
}
```

**Problems:**
1. Says "Select all" but user should only select ONE emoji
2. Uses `target_category` (e.g., "happy") instead of `target_description` (e.g., "smiling or happy face")

**Expected Instruction:** "Click the smiling or happy face"  
**Actual Instruction:** "Select all happy"

**Fix:**
```rust
CaptchaData::Emoji(challenge) => {
    let instruction = format!("Click the <strong>{}</strong>", challenge.target_description);
    render_emoji_captcha_with_message(session_id, challenge, is_threat, title, &instruction)
}
```

---

### Issue 2: ImageRotation CAPTCHA - Generic Message Instead of Specific

**File:** `crates/fortify-gate/src/captcha_html.rs`  
**Lines:** 773-775

**Current Code (BROKEN):**
```rust
CaptchaData::ImageRotation(challenge) => render_image_rotation_captcha_with_message(
    session_id, challenge, is_threat, title, message,
),
```

**Problem:** Passes generic `message` (e.g., "Complete this verification...") instead of shape-specific instruction.

**Expected Instruction:** "Click the **arrow** that is upright"  
**Actual Instruction:** Generic verification message

**Fix:**
```rust
CaptchaData::ImageRotation(challenge) => {
    let instruction = format!(
        "Click the <strong>{}</strong> that is upright",
        challenge.shape_name
    );
    render_image_rotation_captcha_with_message(
        session_id, challenge, is_threat, title, &instruction,
    )
}
```

---

### Issue 3: Silhouette CAPTCHA - Generic Message Instead of Specific

**File:** `crates/fortify-gate/src/captcha_html.rs`  
**Lines:** 776-778

**Current Code (BROKEN):**
```rust
CaptchaData::Silhouette(challenge) => {
    render_silhouette_captcha_with_message(session_id, challenge, is_threat, title, message)
}
```

**Problem:** Passes generic `message` instead of specific instruction.

**Expected Instruction:** "What does this silhouette show?"  
**Actual Instruction:** Generic verification message

**Fix:**
```rust
CaptchaData::Silhouette(challenge) => {
    render_silhouette_captcha_with_message(
        session_id, challenge, is_threat, title, "What does this silhouette show?",
    )
}
```

---

### Issue 4: ImageRotation - Options Not Shuffled (MINOR)

**File:** `crates/fortify-gate/src/captcha_types.rs`  
**Lines:** 941-965

**Current Code:**
```rust
let angles = RotationAngle::all();
let correct_index = 0; // Deg0 is always correct

let options: Vec<RotationOption> = angles
    .iter()
    .enumerate()
    .map(|(i, &angle)| RotationOption {
        rotation: angle,
        index: i,
        display: format!("{} ({}°)", shape_char, angle.degrees()),
    })
    .collect();
```

**Problem:** Options always appear in same order (0°, 90°, 180°, 270°) with correct answer always at index 0. Predictable.

**Fix:** Shuffle the options array and track the new position of the correct answer (Deg0).

---

## Implementation Tasks

| # | Task | File | Status |
|---|------|------|--------|
| 1 | Fix Emoji instruction text | `captcha_html.rs` | ⬜ |
| 2 | Fix ImageRotation instruction text | `captcha_html.rs` | ⬜ |
| 3 | Fix Silhouette instruction text | `captcha_html.rs` | ⬜ |
| 4 | Shuffle ImageRotation options | `captcha_types.rs` | ⬜ |
| 5 | Run `cargo fmt` | - | ⬜ |
| 6 | Run `cargo clippy` | - | ⬜ |
| 7 | Run `cargo test` | - | ⬜ |
| 8 | Create PR and wait for CI | - | ⬜ |

---

## Testing Checklist

- [ ] Emoji CAPTCHA shows "Click the [description]" instruction
- [ ] ImageRotation CAPTCHA shows "Click the [shape] that is upright"
- [ ] Silhouette CAPTCHA shows "What does this silhouette show?"
- [ ] ImageRotation options appear in random order
- [ ] All CAPTCHA types verify correctly when correct answer is selected
- [ ] Tor Browser compatibility (Safest mode) confirmed

---

## Success Criteria

1. All 4 issues fixed
2. All CI checks pass
3. Manual testing confirms correct instructions display
4. Verification success rate improves (no more confusion-based failures)

---

## Files Modified

- `crates/fortify-gate/src/captcha_html.rs` - Fix instruction text for 3 captcha types
- `crates/fortify-gate/src/captcha_types.rs` - Shuffle ImageRotation options
