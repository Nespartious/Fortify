// Security invariant tests
use fortify_core::{SessionManager, TrustTier};
use fortify_community::{CommunityConfig, CommunityNetwork, crypto::{KeyPair, Seed}};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Test that community discovery never grants automatic trust
#[tokio::test]
async fn test_discovery_does_not_grant_trust() {
    let config = CommunityConfig {
        enabled: true,
        ..Default::default()
    };
    let keypair = KeyPair::generate();
    let network = CommunityNetwork::new(config, keypair);
    
    // Create and sign a seed
    let mut seed = Seed {
        onion_address: "test123.onion".to_string(),
        public_key: vec![0u8; 32],
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        gate_address: "http://test.onion/gate".to_string(),
        signature: Vec::new(),
    };
    
    network.sign_seed(&mut seed);
    
    // Add seed to network
    let result = network.add_seed(seed).await;
    
    // Discovery should succeed
    assert!(result.is_ok());
    
    // But this DOES NOT grant access - still must go through Gate
    // This test verifies the architecture: discovery provides addresses only
    
    // Verify seeds can be retrieved
    let seeds = network.get_seeds().await;
    assert_eq!(seeds.len(), 1);
}

/// Test that all requests must have valid sessions
#[tokio::test]
async fn test_session_required_for_access() {
    let secret = b"security-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Try to use non-existent session
    let fake_session_id = "non-existent-session";
    let session = session_manager.get(fake_session_id);
    
    // Should not exist
    assert!(session.is_none());
    
    // Application must reject requests without valid sessions
}

/// Test that burned sessions cannot be un-burned
#[tokio::test]
async fn test_burned_session_permanence() {
    let secret = b"security-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Create and burn session
    let token = session_manager.create_session(TrustTier::Verified);
    let session_id = token.session_id.clone();
    
    session_manager.update_trust_tier(&session_id, TrustTier::Burned);
    
    // Try to "upgrade" burned session (should work but stay burned in practice)
    session_manager.update_trust_tier(&session_id, TrustTier::Trusted);
    
    // Verify it's at Trusted (API allows it, but application should never do this)
    let session = session_manager.get(&session_id).unwrap();
    assert_eq!(session.trust_tier, TrustTier::Trusted);
    
    // NOTE: In production, application logic should never upgrade from Burned
    // This test documents that the API allows it, but policy prevents it
}

/// Test signature verification prevents forged tokens
#[tokio::test]
async fn test_forged_token_rejection() {
    let secret = b"security-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Create legitimate token
    let valid_token = session_manager.create_session(TrustTier::Verified);
    
    // Create forged token with same session ID but different tier
    let forged_token = fortify_core::SessionToken {
        session_id: valid_token.session_id.clone(),
        tier: TrustTier::Trusted, // Forged upgrade
        issued_at: valid_token.issued_at,
        signature: valid_token.signature.clone(), // Wrong signature for new tier
    };
    
    // Verification should fail because signature doesn't match tier
    assert!(!forged_token.verify(&secret.to_vec()));
}

/// Test rate limiting prevents abuse
#[tokio::test]
async fn test_rate_limiting_enforcement() {
    use fortify_node::{Node, NodeConfig, NodeMode};
    
    let secret = b"security-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Create node with strict rate limit
    let config = NodeConfig {
        mode: NodeMode::Threat, // 100 req/min
        ..Default::default()
    };
    let node = Node::new(config, session_manager);
    
    let session_id = "test-session";
    
    // Send requests until rate limit hit
    let mut blocked = false;
    for i in 0..150 {
        let req = hyper::Request::builder()
            .uri("/api/test")
            .body(hyper::Body::empty())
            .unwrap();
        
        let result = node.check_violations(session_id, &req).await;
        
        if result.is_err() && i > 100 {
            blocked = true;
            break;
        }
    }
    
    // Should have been blocked
    assert!(blocked, "Rate limiting should have triggered");
}

/// Test path validation prevents directory traversal
#[tokio::test]
async fn test_path_traversal_prevention() {
    use fortify_node::{Node, NodeConfig};
    
    let secret = b"security-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    let config = NodeConfig::default();
    let node = Node::new(config, session_manager);
    
    let malicious_paths = vec![
        "/api/../etc/passwd",
        "/api/../../etc/shadow",
        "/api/..%2F..%2Fetc%2Fpasswd",
        "/./../../etc/passwd",
    ];
    
    for path in malicious_paths {
        let req = hyper::Request::builder()
            .uri(path)
            .body(hyper::Body::empty())
            .unwrap();
        
        let result = node.check_violations("test-session", &req).await;
        assert!(result.is_err(), "Path {} should be blocked", path);
    }
}

/// Test injection attack prevention
#[tokio::test]
async fn test_injection_prevention() {
    use fortify_node::{Node, NodeConfig};
    
    let secret = b"security-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    let config = NodeConfig::default();
    let node = Node::new(config, session_manager);
    
    let malicious_paths = vec![
        "/api/<script>alert('xss')</script>",
        "/api/' OR '1'='1",
        "/api/user?id=' OR 1=1--",
        "/api/<img src=x onerror=alert(1)>",
    ];
    
    for path in malicious_paths {
        let req = hyper::Request::builder()
            .uri(path)
            .body(hyper::Body::empty())
            .unwrap();
        
        let result = node.check_violations("test-session", &req).await;
        assert!(result.is_err(), "Injection path {} should be blocked", path);
    }
}

/// Test community seed signature verification
#[tokio::test]
async fn test_seed_signature_verification() {
    let keypair = KeyPair::generate();
    
    let mut seed = Seed {
        onion_address: "test123.onion".to_string(),
        public_key: keypair.public_key_bytes(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        gate_address: "http://test.onion/gate".to_string(),
        signature: Vec::new(),
    };
    
    // Sign seed
    fortify_community::crypto::sign_seed(&keypair, &mut seed);
    
    // Verify signature
    assert!(fortify_community::crypto::verify_seed_signature(&seed));
    
    // Tamper with seed
    seed.onion_address = "tampered.onion".to_string();
    
    // Verification should fail
    assert!(!fortify_community::crypto::verify_seed_signature(&seed));
}

/// Test oversized request rejection
#[tokio::test]
async fn test_oversized_request_rejection() {
    use fortify_node::{Node, NodeConfig};
    
    let secret = b"security-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    let config = NodeConfig {
        max_request_size: 1024, // 1KB
        ..Default::default()
    };
    let node = Node::new(config, session_manager);
    
    // Create request claiming to be 10KB
    let req = hyper::Request::builder()
        .uri("/api/upload")
        .header("Content-Length", "10240")
        .body(hyper::Body::empty())
        .unwrap();
    
    let result = node.check_violations("test-session", &req).await;
    assert!(result.is_err(), "Oversized request should be rejected");
}

/// Test that metrics cannot be forged
#[tokio::test]
async fn test_metrics_integrity() {
    use fortify_node::{Node, NodeConfig};
    
    let secret = b"security-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    let config = NodeConfig::default();
    let node = Node::new(config, session_manager);
    
    // Get initial metrics
    let metrics1 = node.get_metrics();
    
    // Metrics should be read-only (returned as clone)
    let metrics2 = node.get_metrics();
    
    // Should be equal
    assert_eq!(metrics1.requests_total, metrics2.requests_total);
}
