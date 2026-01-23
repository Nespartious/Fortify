//! Fuzz target for SessionToken::verify
//!
//! Tests that verifying tokens with arbitrary signatures and payloads
//! never panics. Only returns Ok or Err.

#![no_main]

use fortify_core::{SessionToken, TrustTier};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Scenario 1: Try to verify a token decoded from fuzz data
    if let Ok(token_str) = std::str::from_utf8(data) {
        if let Ok(token) = SessionToken::decode(token_str) {
            // Verify with various secret lengths - should never panic
            let _ = token.verify(b"test_secret_key");
            let _ = token.verify(data); // Fuzzed secret
            let _ = token.verify(&[]); // Empty secret
        }
    }

    // Scenario 2: Create valid-ish token, fuzz the signature bytes
    if data.len() >= 32 {
        let mut token = SessionToken::new(
            "fuzz-session".to_string(),
            TrustTier::Unknown,
            3600,
            "fuzz-user-agent",
        );

        // Replace signature with fuzz data
        token.signature = data[..32].to_vec();

        // Verify should return InvalidSignature, never panic
        let _ = token.verify(b"any_secret");
    }
});
