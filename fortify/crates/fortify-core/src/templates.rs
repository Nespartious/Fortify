//! Template Engine for Fortify HTML Pages
//!
//! This module provides a zero-JavaScript, compile-time embedded template system.
//! Templates are loaded via `include_str!()` for zero disk I/O at runtime.
//!
//! # Design Principles
//! - **No JavaScript**: Tor users distrust JS; all pages are static HTML/CSS
//! - **Compile-time Embedding**: Templates baked into binary for security
//! - **Simple Substitution**: `{{PLACEHOLDER}}` pattern for branding/dynamic content
//! - **Minimal Allocations**: Pre-sized strings where possible

use std::collections::HashMap;

// ============================================================================
// Template Type Enumeration
// ============================================================================

/// All available HTML template types in Fortify
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateType {
    /// Initial gate/landing page shown to new visitors
    Gate,
    /// CAPTCHA challenge page (static template, image injected separately)
    Captcha,
    /// Combined gate + CAPTCHA page for pre-rendered pool (single-page flow)
    GateChallenge,
    /// Generic error page
    Error,
    /// Burned/banned visitor page
    Burned,
    /// Demoted trust tier notification
    Demoted,
    /// Service under maintenance
    Maintenance,
    /// Trust recovery information page
    Recovery,
    /// Mirror retiring soon notice
    Retiring,
    /// Service busy/overloaded page
    Busy,
    /// Main index/welcome page
    Index,
    /// Verification success page
    Verified,
    /// Verification/CAPTCHA failed page
    VerificationFailed,
    /// Session expired/recycled page
    SessionExpired,
}

impl TemplateType {
    /// Returns all template types for iteration
    pub fn all() -> &'static [TemplateType] {
        &[
            TemplateType::Gate,
            TemplateType::Captcha,
            TemplateType::GateChallenge,
            TemplateType::Error,
            TemplateType::Burned,
            TemplateType::Demoted,
            TemplateType::Maintenance,
            TemplateType::Recovery,
            TemplateType::Retiring,
            TemplateType::Busy,
            TemplateType::Index,
            TemplateType::Verified,
            TemplateType::VerificationFailed,
            TemplateType::SessionExpired,
        ]
    }

    /// Returns the filename for this template type
    pub fn filename(&self) -> &'static str {
        match self {
            TemplateType::Gate => "gate.html",
            TemplateType::Captcha => "captcha.html",
            TemplateType::GateChallenge => "gate-challenge.html",
            TemplateType::Error => "error.html",
            TemplateType::Burned => "burned.html",
            TemplateType::Demoted => "demoted.html",
            TemplateType::Maintenance => "maintenance.html",
            TemplateType::Recovery => "recovery.html",
            TemplateType::Retiring => "retiring.html",
            TemplateType::Busy => "busy.html",
            TemplateType::Index => "index.html",
            TemplateType::Verified => "verified.html",
            TemplateType::VerificationFailed => "verification-failed.html",
            TemplateType::SessionExpired => "session-expired.html",
        }
    }
}

// ============================================================================
// Compile-time Template Loading
// ============================================================================

/// Gate/landing page template
pub static TEMPLATE_GATE: &str = include_str!("../../../assets/html/gate.html");

/// CAPTCHA challenge page template
pub static TEMPLATE_CAPTCHA: &str = include_str!("../../../assets/html/captcha.html");

/// Combined Gate + CAPTCHA page template (single-page flow)
pub static TEMPLATE_GATE_CHALLENGE: &str = include_str!("../../../assets/html/gate-challenge.html");

/// Error page template
pub static TEMPLATE_ERROR: &str = include_str!("../../../assets/html/error.html");

/// Burned/banned page template
pub static TEMPLATE_BURNED: &str = include_str!("../../../assets/html/burned.html");

/// Demoted trust tier page template
pub static TEMPLATE_DEMOTED: &str = include_str!("../../../assets/html/demoted.html");

/// Maintenance page template
pub static TEMPLATE_MAINTENANCE: &str = include_str!("../../../assets/html/maintenance.html");

/// Recovery information page template
pub static TEMPLATE_RECOVERY: &str = include_str!("../../../assets/html/recovery.html");

/// Mirror retiring page template
pub static TEMPLATE_RETIRING: &str = include_str!("../../../assets/html/retiring.html");

/// Busy/overloaded page template
pub static TEMPLATE_BUSY: &str = include_str!("../../../assets/html/busy.html");

/// Main index page template
pub static TEMPLATE_INDEX: &str = include_str!("../../../assets/html/index.html");

/// Verification success page template
pub static TEMPLATE_VERIFIED: &str = include_str!("../../../assets/html/verified.html");

/// Verification/CAPTCHA failed page template
pub static TEMPLATE_VERIFICATION_FAILED: &str =
    include_str!("../../../assets/html/verification-failed.html");

/// Session expired/recycled page template
pub static TEMPLATE_SESSION_EXPIRED: &str =
    include_str!("../../../assets/html/session-expired.html");

// ============================================================================
// Template Engine
// ============================================================================

/// Template rendering engine with placeholder substitution
///
/// # Usage
/// ```rust,ignore
/// use fortify_core::templates::{TemplateEngine, TemplateType};
///
/// let engine = TemplateEngine::new();
/// let mut vars = std::collections::HashMap::new();
/// vars.insert("SERVICE_NAME".to_string(), "My Hidden Service".to_string());
/// vars.insert("MIRROR_LIST".to_string(), "<li>mirror1.onion</li>".to_string());
///
/// let html = engine.render(TemplateType::Burned, &vars);
/// ```
#[derive(Debug, Clone)]
pub struct TemplateEngine {
    /// Optional custom CSS to inject into templates
    custom_css: Option<String>,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEngine {
    /// Create a new template engine instance
    pub fn new() -> Self {
        Self { custom_css: None }
    }

    /// Create a template engine with custom CSS injection
    pub fn with_custom_css(css: String) -> Self {
        Self {
            custom_css: Some(css),
        }
    }

    /// Get the raw template content for a template type
    pub fn get_template(&self, template_type: TemplateType) -> &'static str {
        match template_type {
            TemplateType::Gate => TEMPLATE_GATE,
            TemplateType::Captcha => TEMPLATE_CAPTCHA,
            TemplateType::GateChallenge => TEMPLATE_GATE_CHALLENGE,
            TemplateType::Error => TEMPLATE_ERROR,
            TemplateType::Burned => TEMPLATE_BURNED,
            TemplateType::Demoted => TEMPLATE_DEMOTED,
            TemplateType::Maintenance => TEMPLATE_MAINTENANCE,
            TemplateType::Recovery => TEMPLATE_RECOVERY,
            TemplateType::Retiring => TEMPLATE_RETIRING,
            TemplateType::Busy => TEMPLATE_BUSY,
            TemplateType::Index => TEMPLATE_INDEX,
            TemplateType::Verified => TEMPLATE_VERIFIED,
            TemplateType::VerificationFailed => TEMPLATE_VERIFICATION_FAILED,
            TemplateType::SessionExpired => TEMPLATE_SESSION_EXPIRED,
        }
    }

    /// Render a template with variable substitution
    ///
    /// Replaces all `{{KEY}}` placeholders with corresponding values from `vars`.
    /// Unmatched placeholders are left as-is (allows partial rendering).
    ///
    /// # Arguments
    /// * `template_type` - Which template to render
    /// * `vars` - HashMap of placeholder names (without braces) to values
    ///
    /// # Returns
    /// Rendered HTML string with all matched placeholders replaced
    pub fn render(&self, template_type: TemplateType, vars: &HashMap<String, String>) -> String {
        let template = self.get_template(template_type);
        self.render_string(template, vars)
    }

    /// Render an arbitrary template string with variable substitution
    ///
    /// This is useful for rendering sub-templates or custom HTML fragments.
    pub fn render_string(&self, template: &str, vars: &HashMap<String, String>) -> String {
        // Use simple string replacement - efficient for our use case
        // with a small number of placeholders
        let mut output = template.to_string();

        for (key, value) in vars {
            let placeholder = format!("{{{{{}}}}}", key); // {{KEY}}
            output = output.replace(&placeholder, value);
        }

        // Inject custom CSS if configured
        if let Some(ref css) = self.custom_css {
            // Insert before </head> if present
            if let Some(pos) = output.find("</head>") {
                let injection = format!("<style>{}</style>", css);
                output.insert_str(pos, &injection);
            }
        }

        output
    }

    /// Convenience method for rendering with branding variables
    ///
    /// Automatically includes common branding placeholders from BrandingVars
    pub fn render_with_branding(
        &self,
        template_type: TemplateType,
        branding: &BrandingVars,
        extra_vars: Option<&HashMap<String, String>>,
    ) -> String {
        let mut vars = branding.to_hashmap();

        // Merge extra vars if provided
        if let Some(extra) = extra_vars {
            for (k, v) in extra {
                vars.insert(k.clone(), v.clone());
            }
        }

        self.render(template_type, &vars)
    }
}

// ============================================================================
// Branding Variables
// ============================================================================

/// Common branding variables for template rendering
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrandingVars {
    /// Service/site name (e.g., "My Hidden Service")
    pub service_name: String,
    /// Short description of the service
    pub description: String,
    /// Welcome message displayed on gate page
    pub welcome_message: String,
    /// Primary brand color (CSS color value)
    pub primary_color: String,
    /// Secondary brand color
    pub secondary_color: String,
    /// Footer branding text/HTML
    pub footer_branding: String,
    /// Custom CSS injection point content
    pub branding_injection: String,
    /// Gate path for verification (e.g., "/Fortify/Portcullis" or "/gate")
    pub gate_path: String,
}

impl Default for BrandingVars {
    fn default() -> Self {
        Self {
            service_name: "Protected Service".to_string(),
            description: "A Fortify-protected onion service".to_string(),
            welcome_message: "Please complete the verification to continue.".to_string(),
            primary_color: "#c9a227".to_string(),   // Gold
            secondary_color: "#a68b5b".to_string(), // Muted gold
            footer_branding: String::new(),
            branding_injection: String::new(),
            gate_path: "/Fortify/Portcullis".to_string(), // Default gate path
        }
    }
}

impl BrandingVars {
    /// Create BrandingVars from environment variables
    /// Falls back to defaults for any missing values
    pub fn from_env() -> Self {
        Self {
            service_name: std::env::var("BRANDING_SERVICE_NAME")
                .unwrap_or_else(|_| "Protected Service".to_string()),
            description: std::env::var("BRANDING_DESCRIPTION")
                .unwrap_or_else(|_| "A Fortify-protected onion service".to_string()),
            welcome_message: std::env::var("BRANDING_WELCOME_MESSAGE")
                .unwrap_or_else(|_| "Please complete the verification to continue.".to_string()),
            primary_color: std::env::var("BRANDING_PRIMARY_COLOR")
                .unwrap_or_else(|_| "#c9a227".to_string()),
            secondary_color: std::env::var("BRANDING_SECONDARY_COLOR")
                .unwrap_or_else(|_| "#a68b5b".to_string()),
            footer_branding: std::env::var("BRANDING_FOOTER").unwrap_or_default(),
            branding_injection: std::env::var("BRANDING_INJECTION").unwrap_or_default(),
            gate_path: std::env::var("BRANDING_GATE_PATH")
                .unwrap_or_else(|_| "/Fortify/Portcullis".to_string()),
        }
    }

    /// Convert branding vars to HashMap for template rendering
    pub fn to_hashmap(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("SERVICE_NAME".to_string(), self.service_name.clone());
        map.insert("DESCRIPTION".to_string(), self.description.clone());
        map.insert("WELCOME_MESSAGE".to_string(), self.welcome_message.clone());
        map.insert("PRIMARY_COLOR".to_string(), self.primary_color.clone());
        map.insert("SECONDARY_COLOR".to_string(), self.secondary_color.clone());
        map.insert(
            "FORTIFY_FOOTER_BRANDING".to_string(),
            self.footer_branding.clone(),
        );
        map.insert(
            "FORTIFY_BRANDING_INJECTION_POINT".to_string(),
            self.branding_injection.clone(),
        );
        map.insert("GATE_PATH".to_string(), self.gate_path.clone());
        map
    }
}

// ============================================================================
// Pre-rendered CAPTCHA Page
// ============================================================================

/// A pre-rendered CAPTCHA page ready for instant serving
///
/// This struct holds a complete HTML page with the CAPTCHA image already
/// embedded as a data URI. Used by CaptchaPoolManager to pre-generate
/// pages during low-traffic periods.
///
/// Uses the GateChallenge template for single-page verification flow.
#[derive(Debug, Clone)]
pub struct PrerenderedCaptchaPage {
    /// The CAPTCHA challenge ID (for answer verification)
    pub captcha_id: String,
    /// Pre-assigned session ID for this page
    pub session_id: String,
    /// Complete HTML page ready to serve
    pub html: String,
    /// When this page was generated (for staleness checks)
    pub generated_at: u64,
}

impl PrerenderedCaptchaPage {
    /// Create a new pre-rendered CAPTCHA page using the GateChallenge template
    ///
    /// # Arguments
    /// * `captcha_id` - Unique identifier for answer verification
    /// * `session_id` - Pre-assigned session ID for this challenge
    /// * `image_data_uri` - Base64-encoded image as data URI (data:image/png;base64,...)
    /// * `instruction` - Human-readable CAPTCHA instruction (e.g., "Type the characters shown")
    /// * `engine` - Template engine for rendering
    /// * `branding` - Branding variables
    pub fn new(
        captcha_id: String,
        session_id: String,
        image_data_uri: &str,
        instruction: &str,
        engine: &TemplateEngine,
        branding: &BrandingVars,
    ) -> Self {
        let mut vars = branding.to_hashmap();
        vars.insert("CAPTCHA_ID".to_string(), captcha_id.clone());
        vars.insert("SESSION_ID".to_string(), session_id.clone());
        vars.insert("CAPTCHA_IMAGE".to_string(), image_data_uri.to_string());
        vars.insert("CAPTCHA_INSTRUCTION".to_string(), instruction.to_string());

        let html = engine.render(TemplateType::GateChallenge, &vars);

        Self {
            captcha_id,
            session_id,
            html,
            generated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Create a pre-rendered page using the legacy Captcha template
    ///
    /// This is provided for backward compatibility with existing CAPTCHA flows.
    #[allow(dead_code)]
    pub fn new_legacy(
        captcha_id: String,
        image_data_uri: &str,
        engine: &TemplateEngine,
        branding: &BrandingVars,
    ) -> Self {
        let mut vars = branding.to_hashmap();
        vars.insert("CAPTCHA_ID".to_string(), captcha_id.clone());
        vars.insert("CAPTCHA_IMAGE".to_string(), image_data_uri.to_string());

        let html = engine.render(TemplateType::Captcha, &vars);

        Self {
            captcha_id,
            session_id: String::new(),
            html,
            generated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Check if this page is stale (older than max_age_secs)
    pub fn is_stale(&self, max_age_secs: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now.saturating_sub(self.generated_at) > max_age_secs
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_loading() {
        // Verify all templates load without panic
        let engine = TemplateEngine::new();
        for template_type in TemplateType::all() {
            let content = engine.get_template(*template_type);
            assert!(
                !content.is_empty(),
                "Template {:?} should not be empty",
                template_type
            );
            assert!(
                content.contains("<!DOCTYPE html>") || content.contains("<!doctype html>"),
                "Template {:?} should be valid HTML",
                template_type
            );
        }
    }

    #[test]
    fn test_placeholder_substitution() {
        let engine = TemplateEngine::new();
        let template = "Hello, {{NAME}}! Your score is {{SCORE}}.";
        let mut vars = HashMap::new();
        vars.insert("NAME".to_string(), "Alice".to_string());
        vars.insert("SCORE".to_string(), "100".to_string());

        let result = engine.render_string(template, &vars);
        assert_eq!(result, "Hello, Alice! Your score is 100.");
    }

    #[test]
    fn test_unmatched_placeholder_preserved() {
        let engine = TemplateEngine::new();
        let template = "Hello, {{NAME}}! Value: {{UNKNOWN}}.";
        let mut vars = HashMap::new();
        vars.insert("NAME".to_string(), "Bob".to_string());

        let result = engine.render_string(template, &vars);
        assert_eq!(result, "Hello, Bob! Value: {{UNKNOWN}}.");
    }

    #[test]
    fn test_nested_braces_handled() {
        let engine = TemplateEngine::new();
        let template = "Code: { {{VAR}} }";
        let mut vars = HashMap::new();
        vars.insert("VAR".to_string(), "value".to_string());

        let result = engine.render_string(template, &vars);
        assert_eq!(result, "Code: { value }");
    }

    #[test]
    fn test_branding_vars_to_hashmap() {
        let branding = BrandingVars {
            service_name: "TestService".to_string(),
            description: "Test Description".to_string(),
            welcome_message: "Welcome!".to_string(),
            primary_color: "#ff0000".to_string(),
            secondary_color: "#00ff00".to_string(),
            footer_branding: "Footer".to_string(),
            branding_injection: "/* custom */".to_string(),
            gate_path: "/test/gate".to_string(),
        };

        let map = branding.to_hashmap();
        assert_eq!(map.get("SERVICE_NAME").unwrap(), "TestService");
        assert_eq!(map.get("DESCRIPTION").unwrap(), "Test Description");
        assert_eq!(map.get("WELCOME_MESSAGE").unwrap(), "Welcome!");
        assert_eq!(map.get("PRIMARY_COLOR").unwrap(), "#ff0000");
        assert_eq!(map.get("FORTIFY_FOOTER_BRANDING").unwrap(), "Footer");
        assert_eq!(map.get("GATE_PATH").unwrap(), "/test/gate");
    }

    #[test]
    fn test_template_type_all() {
        let all = TemplateType::all();
        assert_eq!(all.len(), 14); // Includes GateChallenge
        assert!(all.contains(&TemplateType::Gate));
        assert!(all.contains(&TemplateType::Captcha));
        assert!(all.contains(&TemplateType::GateChallenge));
        assert!(all.contains(&TemplateType::VerificationFailed));
        assert!(all.contains(&TemplateType::SessionExpired));
        assert!(all.contains(&TemplateType::Verified));
    }

    #[test]
    fn test_template_type_filename() {
        assert_eq!(TemplateType::Gate.filename(), "gate.html");
        assert_eq!(TemplateType::Captcha.filename(), "captcha.html");
        assert_eq!(
            TemplateType::GateChallenge.filename(),
            "gate-challenge.html"
        );
        assert_eq!(TemplateType::Busy.filename(), "busy.html");
    }

    #[test]
    fn test_custom_css_injection() {
        let engine = TemplateEngine::with_custom_css("body { color: red; }".to_string());
        let template = "<!DOCTYPE html><html><head><title>Test</title></head><body></body></html>";
        let vars = HashMap::new();

        let result = engine.render_string(template, &vars);
        assert!(result.contains("<style>body { color: red; }</style></head>"));
    }

    #[test]
    fn test_prerendered_captcha_page() {
        let engine = TemplateEngine::new();
        let branding = BrandingVars::default();

        let page = PrerenderedCaptchaPage::new(
            "test-id-123".to_string(),
            "test-session-456".to_string(),
            "data:image/png;base64,AAAA",
            "Type the characters shown",
            &engine,
            &branding,
        );

        assert_eq!(page.captcha_id, "test-id-123");
        assert_eq!(page.session_id, "test-session-456");
        assert!(!page.html.is_empty());
        assert!(page.html.contains("test-id-123")); // CAPTCHA ID in hidden field
        assert!(page.html.contains("test-session-456")); // Session ID in hidden field
        assert!(page.html.contains("Type the characters shown")); // Instruction
        assert!(!page.is_stale(3600)); // Not stale within 1 hour
    }

    #[test]
    fn test_prerendered_captcha_page_legacy() {
        let engine = TemplateEngine::new();
        let branding = BrandingVars::default();

        let page = PrerenderedCaptchaPage::new_legacy(
            "legacy-id-789".to_string(),
            "data:image/png;base64,BBBB",
            &engine,
            &branding,
        );

        assert_eq!(page.captcha_id, "legacy-id-789");
        assert!(page.session_id.is_empty()); // Legacy has no pre-assigned session
        assert!(!page.html.is_empty());
        assert!(!page.is_stale(3600));
    }
}
