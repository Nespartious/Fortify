//! Fuzz target for IP address extraction from headers
//!
//! Tests the X-Forwarded-For and X-Real-IP parsing logic.
//! These headers come from potentially malicious proxies.

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::net::IpAddr;

/// Extract client IP from X-Forwarded-For header
/// Takes the first IP in the comma-separated list
fn extract_xff_ip(header_value: &str) -> Option<IpAddr> {
    header_value
        .split(',')
        .next()
        .and_then(|ip_str| ip_str.trim().parse().ok())
}

/// Extract client IP from X-Real-IP header
fn extract_real_ip(header_value: &str) -> Option<IpAddr> {
    header_value.trim().parse().ok()
}

/// Validate and extract any IP-like string
fn try_parse_ip(input: &str) -> Option<IpAddr> {
    // Handle various formats attackers might try
    let trimmed = input.trim();

    // Try direct parse first
    if let Ok(ip) = trimmed.parse::<IpAddr>() {
        return Some(ip);
    }

    // Handle bracketed IPv6 (sometimes in URLs)
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let inner = &trimmed[1..trimmed.len() - 1];
        if let Ok(ip) = inner.parse::<IpAddr>() {
            return Some(ip);
        }
    }

    // Handle port suffix (e.g., "192.168.1.1:8080")
    if let Some(ip_part) = trimmed.rsplit(':').last() {
        if let Ok(ip) = ip_part.parse::<IpAddr>() {
            return Some(ip);
        }
    }

    // Handle IPv6 with port (e.g., "[::1]:8080")
    if let Some(bracket_end) = trimmed.find(']') {
        if trimmed.starts_with('[') {
            let inner = &trimmed[1..bracket_end];
            if let Ok(ip) = inner.parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }

    None
}

fuzz_target!(|data: &[u8]| {
    // Test with valid UTF-8
    if let Ok(header_str) = std::str::from_utf8(data) {
        // These should NEVER panic
        let _ = extract_xff_ip(header_str);
        let _ = extract_real_ip(header_str);
        let _ = try_parse_ip(header_str);
    }

    // Test with lossy conversion
    let lossy = String::from_utf8_lossy(data);
    let _ = extract_xff_ip(&lossy);
    let _ = extract_real_ip(&lossy);
    let _ = try_parse_ip(&lossy);

    // Test multi-IP X-Forwarded-For scenarios
    if data.len() > 20 {
        // Simulate comma-separated list
        let parts: Vec<&[u8]> = data.split(|&b| b == b',').collect();
        for part in parts {
            if let Ok(ip_str) = std::str::from_utf8(part) {
                let _ = try_parse_ip(ip_str);
            }
        }
    }

    // Edge cases: various IP-like strings
    let test_cases = [
        "127.0.0.1",
        "::1",
        "256.256.256.256",  // Invalid
        "192.168.1.1:8080",
        "[::1]:8080",
        "",
        " ",
        "localhost",
        "0.0.0.0",
        "255.255.255.255",
        "::",
        "::ffff:192.168.1.1",
    ];

    for case in &test_cases {
        let _ = try_parse_ip(case);
    }
});
