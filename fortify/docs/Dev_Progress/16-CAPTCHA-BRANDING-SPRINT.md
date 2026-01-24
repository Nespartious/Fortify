# Sprint 16: CAPTCHA HTML Branding Unification

## Status: COMPLETE ✅

## Objective
Ensure all CAPTCHA type HTML rendering functions use `BrandingVars::from_env()` instead of hardcoded "FORTIFY" strings, providing consistent branding across all captcha types.

---

## Background

Sprint 14 fixed branding propagation for static HTML pages (gate, busy, error, etc.) via the template engine. However, the dynamically-generated CAPTCHA HTML pages were still using hardcoded "FORTIFY" strings in their `format!()` macros.

### Affected Files
- `fortify-gate/src/captcha_html.rs` - All `render_*_captcha_with_message()` functions

### CAPTCHA Types to Update
| Type | Function | Status |
|------|----------|--------|
| BmpText | `render_bmp_text_captcha_with_message()` | ✅ Already uses TemplateEngine |
| Emoji | `render_emoji_captcha_with_message()` | ✅ Fixed |
| Direction | `render_direction_captcha_with_message()` | ✅ Fixed |
| Sequence | `render_sequence_captcha_with_message()` | ✅ Fixed |
| WordUnscramble | `render_word_unscramble_captcha_with_message()` | ✅ Fixed |
| ImageRotation | `render_image_rotation_captcha_with_message()` | ✅ Fixed |
| Silhouette | `render_silhouette_captcha_with_message()` | ✅ Fixed |

---

## Implementation

### Changes Made

Each CAPTCHA render function was updated to:
1. Import and use `BrandingVars::from_env()`
2. Replace hardcoded `<title>FORTIFY /// ACCESS CONTROL</title>` with `<title>{} — Verification</title>` using `branding.service_name`
3. Replace hardcoded `<h1>FORTIFY</h1>` with `<h1>{}</h1>` using `branding.service_name`

### Code Pattern
```rust
pub fn render_*_captcha_with_message(...) -> String {
    let branding = BrandingVars::from_env();
    // ...
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <title>{} — Verification</title>
    ...
</head>
<body>
    <div class="...">
        <h1>{}</h1>
        ...
    </div>
</body>
</html>"#,
        branding.service_name,
        // ...
        branding.service_name,
        // ...
    )
}
```

---

## Testing Checklist

- [x] `cargo fmt` - Code formatted
- [x] `cargo clippy -- -D warnings` - No warnings
- [x] `cargo test` - All tests pass
- [x] Verified no hardcoded `<h1>FORTIFY</h1>` strings remain in captcha_html.rs

---

## Success Criteria

✅ All 7 CAPTCHA types now use `BrandingVars::from_env()` for service name
✅ Page titles and headings reflect configured branding
✅ No hardcoded "FORTIFY" strings in captcha HTML output

---

## Related

- Sprint 14: Branding Not Applied to HTML Pages (static pages)
- Sprint 15: Config Propagation Audit (ensured env vars flow correctly)
