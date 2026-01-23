use bytes::Bytes;
use fortify_core::{Session, SessionManager, SessionToken, TrustTier};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use std::sync::Arc;

/// Type alias for the response body type used throughout
type BoxBody = Full<Bytes>;

/// Token validation result
pub enum ValidationResult {
    Valid(SessionToken, Session),
    Invalid(Response<BoxBody>),
}

/// Validate request token and session
pub async fn validate_request(
    req: &Request<Incoming>,
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
fn extract_bearer_token(req: &Request<Incoming>) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

/// Create error response
fn error_response(status: StatusCode, message: &str) -> Response<BoxBody> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from(message.to_string())))
        .expect("valid response")
}

#[cfg(test)]
mod tests {
    // Tests require Incoming which can't be easily constructed in unit tests
    // These tests are now better suited for integration testing
}
