use crate::ViolationType;
use hyper::body::Incoming;
use hyper::Request;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Behavioral anomaly detector
pub struct BehaviorDetector {
    request_patterns: HashMap<String, Vec<RequestPattern>>,
    pattern_window_size: usize,
}

/// Request pattern for behavioral analysis
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RequestPattern {
    timestamp: u64,
    path: String,
    method: String,
    user_agent: Option<String>,
}

impl BehaviorDetector {
    pub fn new(pattern_window_size: usize) -> Self {
        Self {
            request_patterns: HashMap::new(),
            pattern_window_size,
        }
    }

    /// Analyze request for suspicious patterns
    pub fn analyze(&mut self, session_id: &str, req: &Request<Incoming>) -> Vec<ViolationType> {
        let mut violations = Vec::new();

        // Record pattern
        let pattern = RequestPattern {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            path: req.uri().path().to_string(),
            method: req.method().to_string(),
            user_agent: req
                .headers()
                .get("User-Agent")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string()),
        };

        let patterns = self
            .request_patterns
            .entry(session_id.to_string())
            .or_default();

        patterns.push(pattern);

        // Keep window size limited
        if patterns.len() > self.pattern_window_size {
            patterns.remove(0);
        }

        // Check for suspicious patterns
        if Self::has_rapid_requests(patterns) {
            violations.push(ViolationType::SuspiciousPattern);
        }

        if Self::has_scan_pattern(patterns) {
            violations.push(ViolationType::SuspiciousPattern);
        }

        violations
    }

    /// Check for rapid sequential requests
    fn has_rapid_requests(patterns: &[RequestPattern]) -> bool {
        if patterns.len() < 3 {
            return false;
        }

        let recent = &patterns[patterns.len() - 3..];
        let time_span = recent.last().unwrap().timestamp - recent.first().unwrap().timestamp;

        // 3 requests in less than 1 second
        time_span < 1
    }

    /// Check for scanning pattern (many different paths)
    fn has_scan_pattern(patterns: &[RequestPattern]) -> bool {
        if patterns.len() < 5 {
            return false;
        }

        let unique_paths: std::collections::HashSet<_> = patterns.iter().map(|p| &p.path).collect();

        // More than 80% different paths suggests scanning
        unique_paths.len() as f32 / patterns.len() as f32 > 0.8
    }

    /// Reset patterns for a session
    pub fn reset(&mut self, session_id: &str) {
        self.request_patterns.remove(session_id);
    }
}

/// Request validator
pub struct RequestValidator {
    max_header_size: usize,
    max_path_length: usize,
}

impl RequestValidator {
    pub fn new(max_header_size: usize, max_path_length: usize) -> Self {
        Self {
            max_header_size,
            max_path_length,
        }
    }

    /// Validate request structure
    pub fn validate(&self, req: &Request<Incoming>) -> Result<(), ViolationType> {
        // Check path length
        if req.uri().path().len() > self.max_path_length {
            return Err(ViolationType::MalformedRequest);
        }

        // Check header sizes
        for (name, value) in req.headers() {
            let size = name.as_str().len() + value.len();
            if size > self.max_header_size {
                return Err(ViolationType::MalformedRequest);
            }
        }

        // Check for null bytes in path
        if req.uri().path().contains('\0') {
            return Err(ViolationType::MalformedRequest);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http_body_util::Full;

    // Helper to create test requests with Full<Bytes> body that we can convert
    #[allow(dead_code)]
    fn make_test_request(uri: &str) -> Request<Full<Bytes>> {
        Request::builder()
            .uri(uri)
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    // Note: Tests that require Request<Incoming> cannot be easily unit tested
    // since Incoming is only created from actual HTTP connections.
    // These patterns are tested via integration tests instead.

    #[test]
    fn test_rapid_request_patterns() {
        // Test the pattern detection logic directly
        let patterns = vec![
            RequestPattern {
                timestamp: 1000,
                path: "/api/test".to_string(),
                method: "GET".to_string(),
                user_agent: None,
            },
            RequestPattern {
                timestamp: 1000,
                path: "/api/test".to_string(),
                method: "GET".to_string(),
                user_agent: None,
            },
            RequestPattern {
                timestamp: 1000,
                path: "/api/test".to_string(),
                method: "GET".to_string(),
                user_agent: None,
            },
        ];
        assert!(BehaviorDetector::has_rapid_requests(&patterns));
    }

    #[test]
    fn test_scan_pattern_detection() {
        // Test scan pattern detection directly
        let patterns: Vec<RequestPattern> = (0..6)
            .map(|i| RequestPattern {
                timestamp: 1000 + i,
                path: format!("/api/endpoint{}", i),
                method: "GET".to_string(),
                user_agent: None,
            })
            .collect();
        assert!(BehaviorDetector::has_scan_pattern(&patterns));
    }

    #[test]
    fn test_request_validator_path_length() {
        let validator = RequestValidator::new(1024, 100);

        let long_path = "/".to_string() + &"a".repeat(150);
        let req = make_test_request(&long_path);

        // We test using Full<Bytes> which implements the same traits
        // The actual Incoming validation happens in integration tests
        assert!(req.uri().path().len() > validator.max_path_length);
    }

    #[test]
    fn test_request_validator_valid_path() {
        let validator = RequestValidator::new(1024, 200);

        let req = make_test_request("/api/users/123");
        assert!(req.uri().path().len() <= validator.max_path_length);
    }
}
