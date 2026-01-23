//! Fuzz target for cookie string parsing
//!
//! Tests the cookie parsing logic that extracts session tokens.
//! Simulates the exact parsing pattern used in fortify-http.

#![no_main]

use libfuzzer_sys::fuzz_target;

/// Replicate the cookie parsing logic from fortify-http
/// This extracts a specific cookie value from a Cookie header
fn parse_cookie_value(cookie_header: &str, cookie_name: &str) -> Option<String> {
    // Split by ; and find the cookie we want
    cookie_header
        .split(';')
        .map(str::trim)
        .find_map(|cookie_pair| {
            // Use split_once to safely split on first '='
            // This avoids index panics on malformed cookies
            let (name, value) = cookie_pair.split_once('=')?;
            if name.trim() == cookie_name {
                Some(value.trim().to_string())
            } else {
                None
            }
        })
}

/// Alternative parsing that handles edge cases
fn parse_all_cookies(cookie_header: &str) -> Vec<(String, String)> {
    cookie_header
        .split(';')
        .filter_map(|pair| {
            let trimmed = pair.trim();
            if trimmed.is_empty() {
                return None;
            }
            // Handle cookies without '=' sign
            match trimmed.split_once('=') {
                Some((name, value)) => Some((name.trim().to_string(), value.trim().to_string())),
                None => Some((trimmed.to_string(), String::new())), // Flag-style cookie
            }
        })
        .collect()
}

fuzz_target!(|data: &[u8]| {
    // Test with valid UTF-8 strings
    if let Ok(cookie_str) = std::str::from_utf8(data) {
        // These should NEVER panic
        let _ = parse_cookie_value(cookie_str, "fortify_session");
        let _ = parse_cookie_value(cookie_str, "");
        let _ = parse_cookie_value(cookie_str, "=");
        let _ = parse_cookie_value(cookie_str, ";");

        let _ = parse_all_cookies(cookie_str);
    }

    // Test with lossy conversion (handles invalid UTF-8)
    let lossy = String::from_utf8_lossy(data);
    let _ = parse_cookie_value(&lossy, "fortify_session");
    let _ = parse_all_cookies(&lossy);

    // Edge case: very long cookie names
    if data.len() > 10 {
        let long_name = String::from_utf8_lossy(&data[..10]);
        let header = String::from_utf8_lossy(data);
        let _ = parse_cookie_value(&header, &long_name);
    }
});
