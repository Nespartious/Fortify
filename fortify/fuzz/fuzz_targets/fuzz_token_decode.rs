//! Fuzz target for SessionToken::decode
//!
//! Tests that decoding arbitrary base64-like strings never panics.
//! This is critical as tokens come from untrusted client cookies.

#![no_main]

use fortify_core::SessionToken;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Try decoding as UTF-8 string first (tokens are base64 text)
    if let Ok(token_str) = std::str::from_utf8(data) {
        // This should NEVER panic - only return Ok or Err
        let _ = SessionToken::decode(token_str);
    }

    // Also try raw base64-like data with various characters
    // that might appear in URL-safe or standard base64
    let as_string = String::from_utf8_lossy(data);
    let _ = SessionToken::decode(&as_string);
});
