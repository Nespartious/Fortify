# Sprint: Branding System & HTML Template Refresh

**Sprint ID:** FEATURE-001  
**Priority:** 🟡 MEDIUM (Enhancement)  
**Estimated Effort:** 3-4 days  
**Status:** ⬜ Not Started  
**Created:** January 22, 2026

---

## Objective

Implement a comprehensive branding system that allows operators to customize:
1. Service name and description
2. Color scheme (primary, secondary, tertiary)
3. Logo placement

All branding variables must be injected into HTML templates with safe defaults.

---

## Current State Analysis

### Existing Branding Configuration
**File:** `crates/fortify-tui/src/config.rs`

```rust
pub struct BrandingConfig {
    pub service_name: String,          // ✅ Exists
    pub description: String,           // ✅ Exists
    pub logo_path: Option<PathBuf>,    // ✅ Exists
    pub logo_base64: Option<String>,   // ✅ Exists
    pub primary_color: String,         // ✅ Exists (hex)
    // ❌ MISSING: secondary_color
    // ❌ MISSING: tertiary_color
    pub custom_css: Option<String>,    // ✅ Exists
    pub welcome_message: String,       // ✅ Exists
}
```

### Current Defaults
```rust
impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            service_name: "Protected Service".to_string(),
            description: "A Fortify-protected onion service".to_string(),
            primary_color: "#6B46C1".to_string(), // Purple
            // ...
        }
    }
}
```

### HTML Templates (Static - No Variables)
| File | Purpose | Needs Branding |
|------|---------|----------------|
| `captcha.html` | CAPTCHA challenge page | ✅ Yes |
| `gate.html` | Initial verification gate | ✅ Yes |
| `error.html` | Error display page | ✅ Yes |
| `burned.html` | Mirror burned notification | ✅ Yes |
| `demoted.html` | Trust demotion notice | ✅ Yes |
| `maintenance.html` | Maintenance mode page | ✅ Yes |
| `recovery.html` | Recovery information | ✅ Yes |
| `retiring.html` | Mirror retirement notice | ✅ Yes |
| `index.html` | Landing page | ✅ Yes |

---

## Implementation Plan

### Phase 1: Extend BrandingConfig

#### Add New Fields
```rust
pub struct BrandingConfig {
    // Existing fields...
    pub service_name: String,
    pub description: String,
    pub logo_path: Option<PathBuf>,
    pub logo_base64: Option<String>,
    pub logo_max_width: u32,
    pub logo_max_height: u32,
    
    // Color palette (hex format: #RRGGBB)
    pub primary_color: String,       // Main brand color
    pub secondary_color: String,     // Accent/highlight color
    pub tertiary_color: String,      // Subtle accent/background
    
    pub custom_css: Option<String>,
    pub welcome_message: String,
}
```

#### Safe Defaults
```rust
impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            service_name: "Protected Service".to_string(),
            description: "A Fortify-protected onion service".to_string(),
            logo_path: None,
            logo_base64: None,
            logo_max_width: 256,
            logo_max_height: 256,
            // Color palette: Dark professional theme
            primary_color: "#6B46C1".to_string(),    // Purple
            secondary_color: "#4A5568".to_string(),   // Slate gray
            tertiary_color: "#2D3748".to_string(),    // Dark slate
            custom_css: None,
            welcome_message: "Please complete the verification to continue.".to_string(),
        }
    }
}
```

#### Validation
```rust
impl BrandingConfig {
    pub fn validate(&self) -> Result<(), BrandingError> {
        // Service name: max 100 characters
        if self.service_name.len() > 100 {
            return Err(BrandingError::ServiceNameTooLong);
        }
        
        // Description: max 100 characters
        if self.description.len() > 100 {
            return Err(BrandingError::DescriptionTooLong);
        }
        
        // Validate hex colors
        for color in [&self.primary_color, &self.secondary_color, &self.tertiary_color] {
            if !Self::is_valid_hex_color(color) {
                return Err(BrandingError::InvalidColor(color.clone()));
            }
        }
        
        Ok(())
    }
    
    fn is_valid_hex_color(color: &str) -> bool {
        color.starts_with('#') && 
        color.len() == 7 && 
        color[1..].chars().all(|c| c.is_ascii_hexdigit())
    }
}
```

---

### Phase 2: Update TUI & Scripts

#### TUI Wizard Updates
**File:** `crates/fortify-tui/src/ui/wizard.rs`

Add input fields:
- [ ] Service Name (existing)
- [ ] Description (existing)
- [ ] Primary Color (existing)
- [ ] Secondary Color (NEW)
- [ ] Tertiary Color (NEW)

#### TUI Settings Tab Updates
**File:** `crates/fortify-tui/src/app.rs`

Add to SettingsTab::Branding:
```rust
SettingsTab::Branding => {
    let fields = vec![
        ("Service Name", self.config.branding.service_name.clone()),
        ("Description", self.config.branding.description.clone()),
        ("Welcome Message", self.config.branding.welcome_message.clone()),
        ("Primary Color", self.config.branding.primary_color.clone()),
        ("Secondary Color", self.config.branding.secondary_color.clone()),
        ("Tertiary Color", self.config.branding.tertiary_color.clone()),
        ("Logo Path", self.config.branding.logo_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default()),
    ];
    // ...
}
```

#### Script Updates
**File:** `install/install.sh`

Add prompts:
```bash
# Branding configuration
read -p "Service Name [Protected Service]: " SERVICE_NAME
SERVICE_NAME=${SERVICE_NAME:-"Protected Service"}

read -p "Description (max 100 chars) []: " DESCRIPTION
DESCRIPTION=${DESCRIPTION:-"A Fortify-protected onion service"}

read -p "Primary Color (hex) [#6B46C1]: " PRIMARY_COLOR
PRIMARY_COLOR=${PRIMARY_COLOR:-"#6B46C1"}

read -p "Secondary Color (hex) [#4A5568]: " SECONDARY_COLOR
SECONDARY_COLOR=${SECONDARY_COLOR:-"#4A5568"}

read -p "Tertiary Color (hex) [#2D3748]: " TERTIARY_COLOR
TERTIARY_COLOR=${TERTIARY_COLOR:-"#2D3748"}
```

---

### Phase 3: HTML Template System

#### Template Variable Format
Use double-brace placeholders:
```html
{{SERVICE_NAME}}
{{DESCRIPTION}}
{{PRIMARY_COLOR}}
{{SECONDARY_COLOR}}
{{TERTIARY_COLOR}}
{{LOGO_BASE64}}
{{WELCOME_MESSAGE}}
```

#### CSS Variable Injection
At the top of each HTML file's `<style>` block:
```html
<style>
    :root {
        /* Brand Colors (injected) */
        --brand-primary: {{PRIMARY_COLOR}};
        --brand-secondary: {{SECONDARY_COLOR}};
        --brand-tertiary: {{TERTIARY_COLOR}};
        
        /* Derived colors */
        --brand-primary-hover: color-mix(in srgb, var(--brand-primary), white 15%);
        --brand-primary-muted: color-mix(in srgb, var(--brand-primary), black 30%);
        
        /* Standard palette (unchanged) */
        --bg-deep: #141417;
        --bg-surface: #1e1e23;
        --text-primary: #f5f0e8;
    }
</style>
```

#### Template Injection Function
```rust
pub fn render_html_template(template: &str, branding: &BrandingConfig) -> String {
    template
        .replace("{{SERVICE_NAME}}", &html_escape(&branding.service_name))
        .replace("{{DESCRIPTION}}", &html_escape(&branding.description))
        .replace("{{PRIMARY_COLOR}}", &branding.primary_color)
        .replace("{{SECONDARY_COLOR}}", &branding.secondary_color)
        .replace("{{TERTIARY_COLOR}}", &branding.tertiary_color)
        .replace("{{WELCOME_MESSAGE}}", &html_escape(&branding.welcome_message))
        .replace("{{LOGO_BASE64}}", branding.logo_base64.as_deref().unwrap_or(""))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&#39;")
}
```

---

### Phase 4: HTML Template Redesign

#### Design Principles
1. **Minimal:** Reduce visual noise
2. **Clean:** Ample whitespace, clear hierarchy
3. **Modern:** Subtle gradients, rounded corners, shadows
4. **Accessible:** High contrast, readable fonts
5. **Lean:** No external dependencies (fonts/CDN)

#### Universal Template Structure
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{SERVICE_NAME}} — {{PAGE_TITLE}}</title>
    <style>
        :root {
            --brand-primary: {{PRIMARY_COLOR}};
            --brand-secondary: {{SECONDARY_COLOR}};
            --brand-tertiary: {{TERTIARY_COLOR}};
            
            --bg-deep: #0a0a0c;
            --bg-surface: #14141a;
            --bg-card: #1a1a22;
            --border-subtle: #2a2a35;
            --text-primary: #f0f0f5;
            --text-secondary: #a0a0aa;
        }
        
        * { box-sizing: border-box; margin: 0; padding: 0; }
        
        body {
            background: linear-gradient(145deg, var(--bg-deep), var(--bg-surface));
            font-family: system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
            color: var(--text-primary);
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            padding: 24px;
        }
        
        .brand-header {
            text-align: center;
            margin-bottom: 32px;
        }
        
        .brand-logo {
            max-width: 80px;
            max-height: 80px;
            margin-bottom: 16px;
        }
        
        .brand-name {
            font-size: 1.5rem;
            font-weight: 600;
            color: var(--brand-primary);
        }
        
        .brand-description {
            font-size: 0.875rem;
            color: var(--text-secondary);
            margin-top: 4px;
        }
        
        .card {
            background: var(--bg-card);
            border: 1px solid var(--border-subtle);
            border-radius: 12px;
            padding: 32px;
            max-width: 480px;
            width: 100%;
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
        }
        
        .btn-primary {
            background: var(--brand-primary);
            color: white;
            border: none;
            padding: 12px 24px;
            border-radius: 8px;
            font-size: 1rem;
            cursor: pointer;
            transition: all 0.2s;
        }
        
        .btn-primary:hover {
            filter: brightness(1.15);
        }
    </style>
</head>
<body>
    <header class="brand-header">
        <!-- Logo (if present) -->
        {{#if LOGO_BASE64}}
        <img src="data:image/png;base64,{{LOGO_BASE64}}" alt="Logo" class="brand-logo">
        {{/if}}
        <h1 class="brand-name">{{SERVICE_NAME}}</h1>
        <p class="brand-description">{{DESCRIPTION}}</p>
    </header>
    
    <main class="card">
        <!-- Page-specific content -->
    </main>
</body>
</html>
```

---

## Implementation Tasks

### Phase 1: Config Updates
- [ ] Add `secondary_color` and `tertiary_color` to `BrandingConfig`
- [ ] Update `Default` implementation with new defaults
- [ ] Add validation for field lengths and color formats
- [ ] Update serialization/deserialization

### Phase 2: TUI Updates
- [ ] Add Secondary Color field to Branding settings tab
- [ ] Add Tertiary Color field to Branding settings tab
- [ ] Add fields to setup wizard Step 2 (Branding)
- [ ] Add color preview in TUI (show color swatch)

### Phase 3: Script Updates
- [ ] Add color prompts to `install/install.sh`
- [ ] Add validation for hex color format in scripts
- [ ] Apply defaults when user presses Enter

### Phase 4: Template Engine
- [ ] Create `render_html_template()` function
- [ ] Add HTML escaping for user-provided strings
- [ ] Create template loading and caching system
- [ ] Integrate with Gate server response handling

### Phase 5: HTML Redesign
- [ ] Convert all templates to use CSS variables
- [ ] Add placeholder syntax to templates
- [ ] Redesign with universal brand header
- [ ] Test all pages with various color schemes

### Phase 6: Testing
- [ ] Test with default branding
- [ ] Test with custom colors (light, dark, vivid)
- [ ] Test with maximum length name/description
- [ ] Test XSS prevention (HTML escaping)
- [ ] Cross-browser testing (Tor Browser, Firefox)

---

## HTML Files to Update

| File | Changes Required |
|------|-----------------|
| `captcha.html` | Add CSS vars, brand header, placeholders |
| `gate.html` | Add CSS vars, brand header, placeholders |
| `error.html` | Add CSS vars, brand header, placeholders |
| `burned.html` | Add CSS vars, brand header, placeholders |
| `demoted.html` | Add CSS vars, brand header, placeholders |
| `maintenance.html` | Add CSS vars, brand header, placeholders |
| `recovery.html` | Add CSS vars, brand header, placeholders |
| `retiring.html` | Add CSS vars, brand header, placeholders |
| `index.html` | Add CSS vars, brand header, placeholders |

---

## Acceptance Criteria

- [ ] All 9 HTML templates support branding variables
- [ ] Default colors produce a professional dark theme
- [ ] Custom colors correctly override defaults
- [ ] Name/description capped at 100 characters with validation
- [ ] HTML escaping prevents XSS in service name/description
- [ ] TUI allows editing all branding fields
- [ ] Scripts prompt for and accept color inputs
- [ ] Templates render correctly in Tor Browser
