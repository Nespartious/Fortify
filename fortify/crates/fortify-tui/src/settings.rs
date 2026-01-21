//! Settings management - loading, saving, validation
//! 
//! This module re-exports types from config and adds settings-specific utilities.

#![allow(dead_code)]
#![allow(unused_imports)]

pub use crate::config::*;

/// Validate a setting value
pub fn validate_setting(field: &str, value: &str) -> Result<(), String> {
    match field {
        "Pool Size" | "Min Mirrors" | "Max Mirrors" | "Standby Mirrors" => {
            value.parse::<usize>()
                .map(|_| ())
                .map_err(|_| "Must be a positive integer".to_string())
        }
        "Difficulty" | "Probe Sensitivity (1-10)" => {
            match value.parse::<u8>() {
                Ok(v) if (1..=10).contains(&v) => Ok(()),
                _ => Err("Must be 1-10".to_string()),
            }
        }
        "Burn Threshold" | "Threat Threshold" | "Suspicion Threshold" => {
            match value.parse::<f32>() {
                Ok(v) if (0.0..=1.0).contains(&v) => Ok(()),
                _ => Err("Must be 0.0-1.0".to_string()),
            }
        }
        "Primary Color" => {
            if value.starts_with('#') && value.len() == 7 {
                Ok(())
            } else {
                Err("Must be hex color (#RRGGBB)".to_string())
            }
        }
        "SOCKS Port" | "Control Port" => {
            match value.parse::<u16>() {
                Ok(v) if v > 0 => Ok(()),
                _ => Err("Must be valid port (1-65535)".to_string()),
            }
        }
        "Backend Address" | "HTTP Bind" | "Gate Bind" => {
            if value.contains(':') {
                Ok(())
            } else {
                Err("Must be address:port format".to_string())
            }
        }
        _ => Ok(()), // Allow other fields
    }
}

/// Get help text for a field
pub fn field_help(field: &str) -> &'static str {
    match field {
        "Service Name" => "Display name for your protected service",
        "Description" => "Short description shown on gate pages",
        "Welcome Message" => "Message shown above CAPTCHA challenge",
        "Primary Color" => "Hex color for branding (#RRGGBB)",
        "Logo Path" => "Path to PNG/JPG logo (max 256x256)",
        "Pool Size" => "Target number of pre-generated CAPTCHAs",
        "Difficulty" => "CAPTCHA difficulty from 1 (easy) to 10 (hard)",
        "Timeout (seconds)" => "Time allowed to solve CAPTCHA",
        "Max Attempts" => "Failed attempts before temporary ban",
        "Rate Limit (req/min)" => "Requests per minute before limiting",
        "Burn Threshold" => "Threat score (0-1) that triggers mirror burn",
        "DDoS RPS Threshold" => "Requests/sec that triggers DDoS protection",
        "Min Mirrors" => "Minimum active mirrors to maintain",
        "Standby Mirrors" => "Warm standbys ready for quick activation",
        "Vanguards Enabled" => "Enable guard discovery protection",
        _ => "",
    }
}
