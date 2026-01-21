// End-to-end tests for complete Fortify flow
use fortify_core::{SessionManager, TrustTier};
use std::sync::Arc;

/// Test complete trust tier progression from Unknown to Trusted
#[tokio::test]
async fn test_trust_tier_progression() {
    let secret = b"e2e-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Create session at Unknown tier (new user)
    let token = session_manager.create_session(TrustTier::Unknown);
    let session_id = token.session_id.clone();
    
    // Verify initial state
    let session = session_manager.get(&session_id).unwrap();
    assert_eq!(session.trust_tier, TrustTier::Unknown);
    
    // Simulate Gate verification → Verified
    session_manager.update_trust_tier(&session_id, TrustTier::Verified);
    let session = session_manager.get(&session_id).unwrap();
    assert_eq!(session.trust_tier, TrustTier::Verified);
    
    // Simulate good behavior in Node → Trusted
    session_manager.update_trust_tier(&session_id, TrustTier::Trusted);
    let session = session_manager.get(&session_id).unwrap();
    assert_eq!(session.trust_tier, TrustTier::Trusted);
}

/// Test trust tier demotion on violations
#[tokio::test]
async fn test_trust_tier_demotion() {
    let secret = b"e2e-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Start at Trusted tier
    let token = session_manager.create_session(TrustTier::Trusted);
    let session_id = token.session_id.clone();
    
    // Demote to Verified
    session_manager.update_trust_tier(&session_id, TrustTier::Verified);
    let session = session_manager.get(&session_id).unwrap();
    assert_eq!(session.trust_tier, TrustTier::Verified);
    
    // Demote to Unknown
    session_manager.update_trust_tier(&session_id, TrustTier::Unknown);
    let session = session_manager.get(&session_id).unwrap();
    assert_eq!(session.trust_tier, TrustTier::Unknown);
    
    // Demote to Suspicious
    session_manager.update_trust_tier(&session_id, TrustTier::Suspicious);
    let session = session_manager.get(&session_id).unwrap();
    assert_eq!(session.trust_tier, TrustTier::Suspicious);
    
    // Burn
    session_manager.update_trust_tier(&session_id, TrustTier::Burned);
    let session = session_manager.get(&session_id).unwrap();
    assert_eq!(session.trust_tier, TrustTier::Burned);
}

/// Test burned session rejection
#[tokio::test]
async fn test_burned_session_rejection() {
    let secret = b"e2e-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Create and burn session
    let token = session_manager.create_session(TrustTier::Verified);
    let session_id = token.session_id.clone();
    
    session_manager.update_trust_tier(&session_id, TrustTier::Burned);
    
    // Verify session exists but is burned
    let session = session_manager.get(&session_id).unwrap();
    assert_eq!(session.trust_tier, TrustTier::Burned);
    
    // Application should reject burned sessions
    assert!(session.trust_tier == TrustTier::Burned);
}

/// Test session token encoding and decoding
#[tokio::test]
async fn test_token_round_trip() {
    let secret = b"e2e-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Create token
    let original_token = session_manager.create_session(TrustTier::Verified);
    
    // Encode
    let encoded = original_token.encode();
    assert!(!encoded.is_empty());
    
    // Decode
    let decoded_token = fortify_core::SessionToken::decode(&encoded);
    assert!(decoded_token.is_ok());
    
    let decoded = decoded_token.unwrap();
    assert_eq!(decoded.session_id, original_token.session_id);
    assert_eq!(decoded.tier, original_token.tier);
    assert_eq!(decoded.signature, original_token.signature);
}

/// Test token tampering detection
#[tokio::test]
async fn test_token_tampering_detection() {
    let secret = b"e2e-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Create valid token
    let token = session_manager.create_session(TrustTier::Verified);
    
    // Encode and tamper
    let encoded = token.encode();
    let tampered = encoded.replace("Verified", "Trusted"); // Try to upgrade tier
    
    // Decode tampered token
    let decoded = fortify_core::SessionToken::decode(&tampered);
    
    // Should either fail to decode or fail verification
    if let Ok(tampered_token) = decoded {
        // Verification should fail
        assert!(!tampered_token.verify(&secret.to_vec()));
    }
}

/// Test concurrent session access
#[tokio::test]
async fn test_concurrent_session_access() {
    use tokio::task::JoinSet;
    
    let secret = b"e2e-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Create session
    let token = session_manager.create_session(TrustTier::Unknown);
    let session_id = token.session_id.clone();
    
    // Spawn multiple tasks accessing same session
    let mut tasks = JoinSet::new();
    
    for i in 0..10 {
        let manager = Arc::clone(&session_manager);
        let id = session_id.clone();
        
        tasks.spawn(async move {
            // Read session
            let session = manager.get(&id);
            assert!(session.is_some());
            
            // Update tier (simulating concurrent updates)
            let new_tier = if i % 2 == 0 {
                TrustTier::Verified
            } else {
                TrustTier::Unknown
            };
            manager.update_trust_tier(&id, new_tier);
        });
    }
    
    // Wait for all tasks
    while let Some(result) = tasks.join_next().await {
        assert!(result.is_ok());
    }
    
    // Session should still exist
    let final_session = session_manager.get(&session_id);
    assert!(final_session.is_some());
}

/// Test session cleanup
#[tokio::test]
async fn test_session_cleanup() {
    let secret = b"e2e-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Create multiple sessions
    let mut session_ids = Vec::new();
    for _ in 0..5 {
        let token = session_manager.create_session(TrustTier::Unknown);
        session_ids.push(token.session_id);
    }
    
    // All should exist
    for id in &session_ids {
        assert!(session_manager.get(id).is_some());
    }
    
    // Delete sessions
    for id in &session_ids {
        session_manager.delete(id);
    }
    
    // All should be gone
    for id in &session_ids {
        assert!(session_manager.get(id).is_none());
    }
}

/// Test multiple tier transitions
#[tokio::test]
async fn test_tier_transition_sequence() {
    let secret = b"e2e-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    let token = session_manager.create_session(TrustTier::Unknown);
    let session_id = token.session_id.clone();
    
    // Transition sequence: Unknown → Verified → Trusted → Verified → Unknown → Suspicious → Burned
    let transitions = vec![
        TrustTier::Unknown,
        TrustTier::Verified,
        TrustTier::Trusted,
        TrustTier::Verified,
        TrustTier::Unknown,
        TrustTier::Suspicious,
        TrustTier::Burned,
    ];
    
    for (i, &tier) in transitions.iter().enumerate() {
        if i > 0 {
            session_manager.update_trust_tier(&session_id, tier);
        }
        
        let session = session_manager.get(&session_id).unwrap();
        assert_eq!(session.trust_tier, tier, "Failed at transition {}", i);
    }
}

/// Test session isolation
#[tokio::test]
async fn test_session_isolation() {
    let secret = b"e2e-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Create two sessions
    let token1 = session_manager.create_session(TrustTier::Unknown);
    let token2 = session_manager.create_session(TrustTier::Verified);
    
    // Update first session
    session_manager.update_trust_tier(&token1.session_id, TrustTier::Trusted);
    
    // Second session should be unchanged
    let session1 = session_manager.get(&token1.session_id).unwrap();
    let session2 = session_manager.get(&token2.session_id).unwrap();
    
    assert_eq!(session1.trust_tier, TrustTier::Trusted);
    assert_eq!(session2.trust_tier, TrustTier::Verified);
}
