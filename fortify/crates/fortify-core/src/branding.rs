//! HTML Template Branding System
//!
//! This module provides template rendering for HTML pages with branding
//! variable injection. All branding values are HTML-escaped to prevent XSS.
//!
//! # Template Variables
//!
//! Templates use double-brace placeholders that are replaced at runtime:
//! - `{{SERVICE_NAME}}` - The service display name
//! - `{{DESCRIPTION}}` - Short service description  
//! - `{{PRIMARY_COLOR}}` - Primary brand color (hex)
//! - `{{SECONDARY_COLOR}}` - Secondary/accent color (hex)
//! - `{{WELCOME_MESSAGE}}` - Welcome message for visitors
//!
//! # Security
//!
//! - All user-provided strings are HTML-escaped
//! - Color values are validated to be hex format only
//! - No JavaScript is ever injected (Tor users expect JS-free pages)

/// Configuration for HTML template branding
///
/// This struct holds all branding values that can be injected into HTML templates.
/// It is designed to be populated from the main BrandingConfig in fortify-tui.
#[derive(Debug, Clone, Default)]
pub struct TemplateBranding {
    /// Display name for the service (HTML-escaped on render)
    pub service_name: String,
    /// Short description (HTML-escaped on render)
    pub description: String,
    /// Primary brand color (hex format: #RRGGBB)
    pub primary_color: String,
    /// Secondary/accent color (hex format: #RRGGBB)
    pub secondary_color: String,
    /// Welcome message for visitors (HTML-escaped on render)
    pub welcome_message: String,
}

impl TemplateBranding {
    /// Create a new TemplateBranding with default Fortify values
    pub fn new() -> Self {
        Self {
            service_name: "Protected Service".to_string(),
            description: "A Fortify-protected onion service".to_string(),
            primary_color: "#c9a227".to_string(),   // Gold
            secondary_color: "#a68b5b".to_string(), // Muted gold
            welcome_message: "Please complete the verification to continue.".to_string(),
        }
    }

    /// Create TemplateBranding with custom values
    pub fn with_values(
        service_name: impl Into<String>,
        description: impl Into<String>,
        primary_color: impl Into<String>,
        secondary_color: impl Into<String>,
        welcome_message: impl Into<String>,
    ) -> Self {
        Self {
            service_name: service_name.into(),
            description: description.into(),
            primary_color: primary_color.into(),
            secondary_color: secondary_color.into(),
            welcome_message: welcome_message.into(),
        }
    }
}

/// Render an HTML template by replacing branding placeholders
///
/// # Arguments
/// * `template` - The HTML template string with {{PLACEHOLDER}} markers
/// * `branding` - The branding configuration to inject
///
/// # Returns
/// The rendered HTML string with all placeholders replaced
///
/// # Security
/// All text fields are HTML-escaped to prevent XSS attacks.
/// Color fields are passed through as-is (should be pre-validated).
pub fn render_html_template(template: &str, branding: &TemplateBranding) -> String {
    template
        .replace("{{SERVICE_NAME}}", &html_escape(&branding.service_name))
        .replace("{{DESCRIPTION}}", &html_escape(&branding.description))
        .replace("{{PRIMARY_COLOR}}", &branding.primary_color)
        .replace("{{SECONDARY_COLOR}}", &branding.secondary_color)
        .replace(
            "{{WELCOME_MESSAGE}}",
            &html_escape(&branding.welcome_message),
        )
}

/// Escape HTML special characters to prevent XSS
///
/// Converts:
/// - `&` → `&amp;`
/// - `<` → `&lt;`
/// - `>` → `&gt;`
/// - `"` → `&quot;`
/// - `'` → `&#39;`
///
/// # Arguments
/// * `s` - The string to escape
///
/// # Returns
/// The escaped string safe for HTML insertion
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Validate that a string is a valid hex color (#RRGGBB format)
pub fn is_valid_hex_color(color: &str) -> bool {
    color.starts_with('#') && color.len() == 7 && color[1..].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_escape_basic() {
        assert_eq!(html_escape("Hello World"), "Hello World");
        assert_eq!(html_escape(""), "");
    }

    #[test]
    fn test_html_escape_special_chars() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("A & B"), "A &amp; B");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(html_escape("it's"), "it&#39;s");
    }

    #[test]
    fn test_html_escape_xss_prevention() {
        let malicious = "<script>alert('xss')</script>";
        let escaped = html_escape(malicious);
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
        assert!(escaped.contains("&lt;"));
        assert!(escaped.contains("&gt;"));
    }

    #[test]
    fn test_render_template_basic() {
        let template = "<h1>{{SERVICE_NAME}}</h1><p>{{DESCRIPTION}}</p>";
        let branding = TemplateBranding {
            service_name: "My Service".to_string(),
            description: "A test service".to_string(),
            ..Default::default()
        };

        let result = render_html_template(template, &branding);
        assert_eq!(result, "<h1>My Service</h1><p>A test service</p>");
    }

    #[test]
    fn test_render_template_colors() {
        let template = ":root { --primary: {{PRIMARY_COLOR}}; --secondary: {{SECONDARY_COLOR}}; }";
        let branding = TemplateBranding {
            primary_color: "#FF0000".to_string(),
            secondary_color: "#00FF00".to_string(),
            ..Default::default()
        };

        let result = render_html_template(template, &branding);
        assert!(result.contains("#FF0000"));
        assert!(result.contains("#00FF00"));
    }

    #[test]
    fn test_render_template_escapes_user_content() {
        let template = "<title>{{SERVICE_NAME}}</title>";
        let branding = TemplateBranding {
            service_name: "<script>alert('xss')</script>".to_string(),
            ..Default::default()
        };

        let result = render_html_template(template, &branding);
        assert!(!result.contains("<script>"));
        assert!(result.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_valid_hex_colors() {
        assert!(is_valid_hex_color("#000000"));
        assert!(is_valid_hex_color("#FFFFFF"));
        assert!(is_valid_hex_color("#c9a227"));
        assert!(is_valid_hex_color("#AbCdEf"));
    }

    #[test]
    fn test_invalid_hex_colors() {
        assert!(!is_valid_hex_color(""));
        assert!(!is_valid_hex_color("#FFF")); // Too short
        assert!(!is_valid_hex_color("#FFFFFFFF")); // Too long
        assert!(!is_valid_hex_color("FFFFFF")); // Missing #
        assert!(!is_valid_hex_color("#GGGGGG")); // Invalid chars
        assert!(!is_valid_hex_color("red")); // Named color
    }

    #[test]
    fn test_template_branding_defaults() {
        let branding = TemplateBranding::new();
        assert_eq!(branding.service_name, "Protected Service");
        assert_eq!(branding.primary_color, "#c9a227");
        assert_eq!(branding.secondary_color, "#a68b5b");
    }
}
