use fortify_core::{Session, SessionManager, SessionToken, TrustTier};
use hyper::{Body, Request, Response, StatusCode};
use std::sync::Arc;

/// Token validation result
pub enum ValidationResult {
    Valid(SessionToken, Session),
    Invalid(Response<Body>),
}

/// Validate request token and session
pub async fn validate_request(
    req: &Request<Body>,
    secret_key: &[u8],
    session_manager: Arc<SessionManager>,
) -> ValidationResult {
    // Extract token
    let token_str = match extract_bearer_token(req) {
        Some(t) => t,
        None => {
            return ValidationResult::Invalid(error_response(
                StatusCode::UNAUTHORIZED,
                "Missing Authorization header with Bearer token",
            ));
        }
    };

    // Decode token
    let token = match SessionToken::decode(&token_str) {
        Ok(t) => t,
        Err(_) => {
            return ValidationResult::Invalid(error_response(
                StatusCode::UNAUTHORIZED,
                "Invalid token format",
            ));
        }
    };

    // Verify signature
    if token.verify(secret_key).is_err() {
        return ValidationResult::Invalid(error_response(
            StatusCode::UNAUTHORIZED,
            "Invalid token signature",
        ));
    }

    // Check expiration
    if !token.is_valid() {
        return ValidationResult::Invalid(error_response(
            StatusCode::UNAUTHORIZED,
            "Token expired",
        ));
    }

    // Get or create session
    let session = match session_manager.get_session(&token.session_id) {
        Some(s) => s,
        None => {
            let mut session = session_manager.create_session(token.session_id.clone());
            session.token.trust_tier = token.trust_tier;
            session_manager.update_session(session.clone());
            session
        }
    };

    // Check if burned
    if session.token.trust_tier == TrustTier::Burned {
        return ValidationResult::Invalid(error_response(
            StatusCode::FORBIDDEN,
            "Session has been burned",
        ));
    }

    ValidationResult::Valid(token, session)
}

/// Extract Bearer token from Authorization header
fn extract_bearer_token(req: &Request<Body>) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

/// Create error response
fn error_response(status: StatusCode, message: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(Body::from(message.to_string()))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bearer_token() {
        let req = Request::builder()
            .header("Authorization", "Bearer my-token-123")
            .body(Body::empty())
            .unwrap();

        assert_eq!(extract_bearer_token(&req), Some("my-token-123".to_string()));
    }

    #[test]
    fn test_extract_bearer_token_no_header() {
        let req = Request::builder().body(Body::empty()).unwrap();

        assert_eq!(extract_bearer_token(&req), None);
    }

    #[test]
    fn test_extract_bearer_token_wrong_scheme() {
        let req = Request::builder()
            .header("Authorization", "Basic dXNlcjpwYXNz")
            .body(Body::empty())
            .unwrap();

        assert_eq!(extract_bearer_token(&req), None);
    }

    #[tokio::test]
    async fn test_validate_request_missing_token() {
        let secret = b"test-secret";
        let session_manager = Arc::new(SessionManager::new(secret.to_vec()));
        let req = Request::builder().body(Body::empty()).unwrap();

        match validate_request(&req, secret, session_manager).await {
            ValidationResult::Invalid(response) => {
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            }
            ValidationResult::Valid(_, _) => panic!("Expected invalid result"),
        }
    }

    #[tokio::test]
    async fn test_validate_request_valid_token() {
        let secret = b"test-secret-key-123";
        let session_manager = Arc::new(SessionManager::new(secret.to_vec()));

        // Create a valid token
        let mut token = SessionToken::new(
            "session-123".into(),
            TrustTier::Verified,
            3600,
            "test-agent",
        );
        token.sign(secret).unwrap();
        let token_str = token.encode().unwrap();

        // Build request with token
        let req = Request::builder()
            .header("Authorization", format!("Bearer {}", token_str))
            .body(Body::empty())
            .unwrap();

        match validate_request(&req, secret, Arc::clone(&session_manager)).await {
            ValidationResult::Valid(validated_token, session) => {
                assert_eq!(validated_token.session_id, "session-123");
                assert_eq!(session.token.trust_tier, TrustTier::Verified);
            }
            ValidationResult::Invalid(_) => panic!("Expected valid result"),
        }
    }
}
