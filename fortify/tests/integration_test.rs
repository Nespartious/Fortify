// Integration tests for Fortify components
use fortify_core::{SessionManager, TrustTier};
use fortify_gate::{Gate, GateConfig, VerificationState};
use fortify_http::{HttpProxy, ProxyConfig};
use fortify_node::{Node, NodeConfig, NodeMode};
use hyper::{Body, Request, Response, StatusCode};
use std::sync::Arc;

/// Test Gate to HTTP Proxy token flow
#[tokio::test]
async fn test_gate_to_proxy_token_flow() {
    // Setup shared session manager
    let secret = b"integration-test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Create Gate
    let gate_config = GateConfig::default();
    let gate = Gate::new(gate_config, Arc::clone(&session_manager));
    
    // Simulate Gate verification (normally requires captcha + PoW)
    let token = session_manager.create_session(TrustTier::Verified);
    
    // Verify token structure
    assert!(!token.session_id.is_empty());
    assert!(!token.signature.is_empty());
    assert_eq!(token.tier, TrustTier::Verified);
    
    // Create HTTP Proxy
    let proxy_config = ProxyConfig::default();
    let proxy = HttpProxy::new(proxy_config, Arc::clone(&session_manager));
    
    // Build request with token
    let req = Request::builder()
        .uri("/api/test")
        .header("Authorization", format!("Bearer {}", token.encode()))
        .body(Body::empty())
        .unwrap();
    
    // Proxy should validate token
    let validation_result = fortify_http::middleware::validate_request(
        &req,
        Arc::clone(&session_manager),
    );
    
    assert!(validation_result.is_ok());
    let validated_session_id = validation_result.unwrap();
    assert_eq!(validated_session_id, token.session_id);
}

/// Test session creation and retrieval
#[tokio::test]
async fn test_session_lifecycle() {
    let secret = b"test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Create session
    let token = session_manager.create_session(TrustTier::Unknown);
    let session_id = token.session_id.clone();
    
    // Retrieve session
    let session = session_manager.get(&session_id);
    assert!(session.is_some());
    
    let session = session.unwrap();
    assert_eq!(session.trust_tier, TrustTier::Unknown);
    assert_eq!(session.id, session_id);
    
    // Update trust tier
    session_manager.update_trust_tier(&session_id, TrustTier::Verified);
    
    let updated_session = session_manager.get(&session_id).unwrap();
    assert_eq!(updated_session.trust_tier, TrustTier::Verified);
}

/// Test token signature verification
#[tokio::test]
async fn test_token_signature_verification() {
    let secret = b"test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Create valid token
    let token = session_manager.create_session(TrustTier::Verified);
    
    // Verify valid token
    assert!(token.verify(&secret.to_vec()));
    
    // Verify fails with wrong secret
    let wrong_secret = b"wrong-secret";
    assert!(!token.verify(&wrong_secret.to_vec()));
}

/// Test Node violation detection
#[tokio::test]
async fn test_node_violation_detection() {
    let secret = b"test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Create session
    let token = session_manager.create_session(TrustTier::Verified);
    let session_id = token.session_id.clone();
    
    // Create Node in Threat mode (100 req/min limit)
    let config = NodeConfig {
        mode: NodeMode::Threat,
        ..Default::default()
    };
    let node = Node::new(config, Arc::clone(&session_manager));
    
    // Send 101 requests rapidly - should hit rate limit
    for i in 0..101 {
        let req = Request::builder()
            .uri("/api/test")
            .body(Body::empty())
            .unwrap();
        
        let result = node.check_violations(&session_id, &req).await;
        
        if i < 100 {
            assert!(result.is_ok(), "Request {} should succeed", i);
        } else {
            assert!(result.is_err(), "Request 101 should fail");
        }
    }
}

/// Test Node session demotion on violations
#[tokio::test]
async fn test_node_session_demotion() {
    let secret = b"test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    
    // Create session at Verified tier
    let token = session_manager.create_session(TrustTier::Verified);
    let session_id = token.session_id.clone();
    
    // Verify initial tier
    let session = session_manager.get(&session_id).unwrap();
    assert_eq!(session.trust_tier, TrustTier::Verified);
    
    // Create Node with low violation threshold
    let config = NodeConfig {
        violation_threshold: 3,
        ..Default::default()
    };
    let node = Node::new(config, Arc::clone(&session_manager));
    
    // Record 3 violations
    for _ in 0..3 {
        node.record_violation(&session_id, fortify_node::ViolationType::RateLimitExceeded).await;
    }
    
    // Should be demoted to Unknown
    let demoted_session = session_manager.get(&session_id).unwrap();
    assert_eq!(demoted_session.trust_tier, TrustTier::Unknown);
}

/// Test Proxy backend selection strategies
#[tokio::test]
async fn test_proxy_routing_strategies() {
    use fortify_http::routing::{Router, BackendNode};
    
    let backends = vec![
        BackendNode::new("http://127.0.0.1:8081".to_string(), 1.0),
        BackendNode::new("http://127.0.0.1:8082".to_string(), 1.0),
        BackendNode::new("http://127.0.0.1:8083".to_string(), 1.0),
    ];
    
    // Test Round Robin
    let mut router = Router::round_robin(backends.clone());
    
    let first = router.select_backend();
    let second = router.select_backend();
    let third = router.select_backend();
    let fourth = router.select_backend();
    
    // Should cycle through backends
    assert_ne!(first, second);
    assert_ne!(second, third);
    assert_eq!(first, fourth); // Back to first
}

/// Test Gate verification state machine
#[tokio::test]
async fn test_gate_verification_flow() {
    let secret = b"test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    let config = GateConfig::default();
    let gate = Gate::new(config, Arc::clone(&session_manager));
    
    // Start verification
    let session_id = "test-session".to_string();
    let challenge = gate.start_verification(&session_id).await;
    
    assert!(challenge.is_ok());
    
    // Check state is AwaitingCaptcha
    let state = gate.get_verification_state(&session_id).await;
    assert!(matches!(state, Some(VerificationState::AwaitingCaptcha(_))));
}

#[tokio::test]
async fn test_invalid_path_detection() {
    let secret = b"test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    let config = NodeConfig::default();
    let node = Node::new(config, session_manager);
    
    // Test path traversal attempt
    let req = Request::builder()
        .uri("/api/../etc/passwd")
        .body(Body::empty())
        .unwrap();
    
    let result = node.check_violations("test-session", &req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_script_injection_detection() {
    let secret = b"test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    let config = NodeConfig::default();
    let node = Node::new(config, session_manager);
    
    // Test script injection attempt
    let req = Request::builder()
        .uri("/api/<script>alert('xss')</script>")
        .body(Body::empty())
        .unwrap();
    
    let result = node.check_violations("test-session", &req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_oversized_request_detection() {
    let secret = b"test-secret";
    let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
    let config = NodeConfig {
        max_request_size: 1024, // 1KB limit
        ..Default::default()
    };
    let node = Node::new(config, session_manager);
    
    // Request claiming to be 2KB
    let req = Request::builder()
        .uri("/api/upload")
        .header("Content-Length", "2048")
        .body(Body::empty())
        .unwrap();
    
    let result = node.check_violations("test-session", &req).await;
    assert!(result.is_err());
}
